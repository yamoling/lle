use pyo3::{exceptions::PyValueError, prelude::*};
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pymethods};

use crate::{
    bindings::{PyAction, PyWorld},
    solver::{Clause, ClauseGenerator, Literal},
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

    /// Generate the literal values assignments that corresponds to the assumption that no cooperation
    /// ever occurs at time step `t`.
    fn assume_no_cooperation(&mut self, t: usize) -> Vec<Literal> {
        self.inner.assume_no_cooperation(t)
    }

    /// Generate the cooperation-tracking clauses for time step `t`: the `laser_blocked` and
    /// `coop_event` indicator-variable definitions.
    ///
    /// These are additive to `generate(t)` (they introduce new variables referencing the same
    /// per-step agent variables) and are only needed when reasoning about who helps whom, e.g.
    /// by `lle.cooperation.characterize`.
    fn cooperation_clauses(&mut self, t: usize) -> Vec<Clause> {
        self.inner.cooperation_clauses(t)
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
