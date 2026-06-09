use pyo3::{exceptions::PyValueError, prelude::*};
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pymethods};

use crate::{
    bindings::{PyAction, PyWorld, pyexceptions::solver_error_to_exception},
    solver::{Clause, ClauseGenerator},
};

/// Generates the SAT clauses (CNF, as lists of signed integer literals) used by
/// `lle.solver.solve` and decodes solver models back into joint-action plans.
///
/// The constraint generation itself (agent movement, collisions, laser propagation
/// and blocking, objective) is implemented in Rust for performance; SAT solving
/// remains delegated to Python (e.g. `pysat.solvers.Minisat22`).
///
/// ```python
/// from pysat.solvers import Minisat22
/// from lle import World
/// from lle.solver.constraints import ClauseGenerator
///
/// world = World.level(1)
/// gen = ClauseGenerator(world, t_max=20)
/// clauses = [c for t in range(gen.t_max + 1) for c in gen.generate(t)]
/// with Minisat22(bootstrap_with=clauses) as solver:
///     solver.append_formula(gen.objective(gen.t_max))
///     if solver.solve():
///         plan = gen.decode_plan(solver.get_model(), gen.t_max)
/// ```
#[gen_stub_pyclass]
#[pyclass(name = "ClauseGenerator", module = "lle.solver.constraints")]
pub struct PyClauseGenerator {
    inner: ClauseGenerator,
    /// The maximum time step considered by this generator.
    #[pyo3(get)]
    t_max: usize,
    /// A cheap admissible lower bound on the length of any valid plan: the maximum,
    /// over all agents, of the shortest walkable-path distance to the nearest exit.
    #[pyo3(get)]
    solution_lower_bound: usize,
}

#[gen_stub_pymethods]
#[pymethods]
impl PyClauseGenerator {
    /// Build a clause generator for the given `world`, considering plans of length up to `t_max`.
    #[new]
    fn new(world: &PyWorld, t_max: usize) -> Self {
        let inner = world.with_world(|world| ClauseGenerator::new(world, t_max));
        let solution_lower_bound = inner.solution_lower_bound();
        Self {
            inner,
            t_max,
            solution_lower_bound,
        }
    }

    /// Generate every clause (initialization, movement and laser constraints) that applies
    /// at time step `t`. Clauses are returned as lists of signed integer literals (CNF),
    /// ready to be fed to a `pysat` solver.
    fn generate(&mut self, t: usize) -> Vec<Clause> {
        self.inner.generate(t)
    }

    /// Generate the objective clauses for time step `t`: every agent must be on an exit.
    /// Intended to be appended to the formula of the solver instance considering `t_end = t`.
    fn objective(&mut self, t: usize) -> Vec<Clause> {
        self.inner.objective(t)
    }

    /// Generate the unit clauses implementing strict (no-cooperation) laser mode at time `t`:
    /// since beams can never be blocked, every beam tile is permanently active, so no agent of a
    /// *different* colour may ever stand on one. The laser's own colour is immune and may still
    /// walk through its own beam.
    ///
    /// Adding these clauses for every `t` in `[0, t_max]` makes the formula UNSAT iff laser
    /// blocking (cooperation) is required to solve the level within the horizon.
    fn no_blocking_clauses(&mut self, t: usize) -> Vec<Clause> {
        self.inner.no_blocking_clauses(t)
    }

    /// Generate the unit clauses assuming no cooperation is required between agents.
    ///
    /// # Raises
    ///     - `SolverError` if the cooperation variables are not yet created.
    fn assume_no_cooperation(&mut self, t_min: usize, t_max: usize) -> PyResult<Vec<Clause>> {
        self.inner
            .assume_no_cooperation(t_min, t_max)
            .map_err(solver_error_to_exception)
    }

    /// Generate the cooperation-tracking clauses for time step `t`: the `laser_blocked` and
    /// `coop_event` indicator-variable definitions.
    ///
    /// These are additive to `generate(t)` (they introduce new variables referencing the same
    /// per-step agent variables) and are only needed when reasoning about who helps whom, e.g.
    /// by `lle.cooperation.characterize`.
    fn coop_clauses(&mut self, t: usize) -> Vec<Clause> {
        self.inner.coop_clauses(t)
    }

    /// Generate the `depends_on(beneficiary, helper)` definition clauses over the horizon
    /// `[0, t_end]`.
    ///
    /// Call this once per candidate horizon, after `coop_clauses(t)` has been generated for every
    /// `t` in `[0, t_end]`. The clauses depend on `t_end`, so feed them to the solver for that
    /// horizon only; do not accumulate them across horizons.
    fn finalize_depends_on(&mut self, t_end: usize) -> Vec<Clause> {
        self.inner.finalize_depends_on(t_end)
    }

    /// The SAT literal for `depends_on(beneficiary, helper)`, or `None` when no such variable
    /// exists (the dependency can never occur within the horizon last passed to
    /// `finalize_depends_on`). Never creates a variable, so the returned literal is safe to use as
    /// a solver assumption.
    fn depends_on_lit(&self, beneficiary: usize, helper: usize) -> Option<i32> {
        self.inner.depends_on_lit(beneficiary, helper)
    }

    /// Generate the temporal chain-tracking clauses for time step `t`.
    ///
    /// Must be called after `coop_clauses(t)` for the same `t`. Defines:
    ///
    /// - `first_helped_by_time(a, b, t)` — "a has helped b at any time ≤ t" (running OR)
    /// - `chain_event(a, b, c, t)` — "a helped b at some time ≤ t-1 AND b helps c at t"
    ///
    /// These are used by `finalize_chain` to define the per-horizon `chain(a, b, c)` variables.
    fn chain_clauses(&mut self, t: usize) -> Vec<Clause> {
        self.inner.chain_clauses(t)
    }

    /// Generate `chain(a, b, c)` definition clauses over the horizon `[0, t_end]`.
    ///
    /// Call once per candidate horizon, after `chain_clauses(t)` has been called for every
    /// `t ≤ t_end`. Feed the result to the solver for that horizon only.
    fn finalize_chain(&mut self, t_end: usize) -> Vec<Clause> {
        self.inner.finalize_chain(t_end)
    }

    /// The SAT literal for `chain(a, b, c)` — "a helped b strictly before b helped c" — or
    /// `None` if no such chain can occur within the horizon last passed to `finalize_chain`.
    fn chain_lit(&self, a: usize, b: usize, c: usize) -> Option<i32> {
        self.inner.chain_lit(a, b, c)
    }

    /// Generate `mutual(a, b)` definition clauses for the current horizon.
    ///
    /// Must be called after `finalize_depends_on(t_end)` for the same horizon (it reads the
    /// `depends_on` variables created by that call). Feed the result to the solver together
    /// with the `finalize_depends_on` output.
    fn finalize_mutual(&mut self, t_end: usize) -> Vec<Clause> {
        self.inner.finalize_mutual(t_end)
    }

    /// The SAT literal for `mutual(a, b)` — "agents a and b mutually depend on each other" —
    /// or `None` if at least one direction of help is impossible within the current horizon.
    fn mutual_lit(&self, a: usize, b: usize) -> Option<i32> {
        self.inner.mutual_lit(a, b)
    }

    /// Decode a SAT model (as returned by `solver.get_model()`) into a joint-action plan
    /// of length `t_end`, i.e. a list of `t_end` joint actions (one action per agent).
    ///
    /// Raises:
    ///     `ValueError`: if the model does not encode a coherent sequence of moves.
    fn decode_plan(&self, model: Vec<i32>, t_end: usize) -> PyResult<Vec<Vec<PyAction>>> {
        match self.inner.decode_plan(&model, t_end) {
            Ok(plan) => Ok(plan
                .into_iter()
                .map(|joint| joint.iter().map(PyAction::from).collect())
                .collect()),
            Err(e) => Err(PyValueError::new_err(e)),
        }
    }
}
