use pyo3::{exceptions::PyValueError, prelude::*};
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pymethods};

use super::pysolvemode::PySolveMode;
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
/// The `mode` parameter controls which extra constraints are generated:
/// - `"standard"` (default): world rules only.
/// - `"no-cooperation"`: assumptions forbidding any non-owner agent from entering a laser span.
/// - `"no-mutual-cooperation"`: clauses and assumptions forbidding pairs of agents from
///   mutually helping each other.
///
/// ```python
/// from pysat.solvers import Minisat22
/// from lle import World
/// from lle.solver.constraints import ClauseGenerator
///
/// world = World.level(1)
/// gen = ClauseGenerator(world, t_max=20)
/// for t in range(gen.solution_lower_bound, gen.t_max + 1):
///     clauses, assumptions = gen.generate(t)
///     with Minisat22(bootstrap_with=clauses) as solver:
///         if solver.solve(assumptions=assumptions):
///             plan = gen.decode_plan(solver.get_model(), t)
///             break
/// ```
#[gen_stub_pyclass]
#[pyclass(name = "ClauseGenerator", module = "lle.solver.constraints")]
pub struct PyClauseGenerator {
    inner: ClauseGenerator,
    /// The maximum time step considered by this generator.
    #[pyo3(get)]
    t_max: usize,
    /// A cheap admissible lower bound on the length of any valid plan: the maximum,
    /// over all agents, of the shortest walkable-path distance to the nearest exit
    /// regardless of lasers.
    #[pyo3(get)]
    solution_lower_bound: usize,
}

#[gen_stub_pymethods]
#[pymethods]
impl PyClauseGenerator {
    /// Build a clause generator for the given `world`, considering plans of length up to `t_max`.
    ///
    /// `mode` selects the solving strategy. It accepts either a `SolveMode` instance or a raw
    /// string literal (`"standard"`, `"no-cooperation"`, `"no-mutual-cooperation"`). Defaults to
    /// `SolveMode.STANDARD`.
    #[new]
    fn new(world: &PyWorld, t_max: usize, mode: PySolveMode) -> PyResult<Self> {
        let inner = world.with_world(|world| ClauseGenerator::new(world, t_max, mode.into()));
        let solution_lower_bound = inner.solution_lower_bound();
        Ok(Self {
            inner,
            t_max,
            solution_lower_bound,
        })
    }

    /// Generate all clauses and assumptions required to solve the problem at horizon `t`.
    ///
    /// Buffers world-enforcing clauses incrementally (each step is computed at most once),
    /// then returns the full formula for this horizon:
    /// - All buffered clauses for steps `0..=t`
    /// - The objective clauses (every agent on an exit at step `t`)
    /// - For `"no-mutual-cooperation"` mode: mutual-forbid clauses and assumptions
    /// - For `"no-cooperation"` mode: per-step no-cooperation assumptions
    ///
    /// Returns `(clauses, assumptions)` ready to be fed to `solve_model`.
    fn generate(&mut self, t: usize) -> (Vec<Clause>, Vec<Literal>) {
        self.inner.generate(t)
    }

    /// Generate only the objective clauses for horizon `t` (every agent on an exit).
    ///
    /// Returns `(clauses, [])`. Useful for callers that manage the SAT solver directly
    /// and want to append the objective separately.
    fn objective(&mut self, t: usize) -> (Vec<Clause>, Vec<Literal>) {
        (self.inner.objective(t), vec![])
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
