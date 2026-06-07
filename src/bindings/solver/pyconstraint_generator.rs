use pyo3::{exceptions::PyValueError, prelude::*};
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pymethods};

use crate::{
    bindings::{PyAction, PyWorld},
    solver::ConstraintGenerator,
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
/// from lle.solver.constraints import ConstraintGenerator
///
/// world = World.level(1)
/// gen = ConstraintGenerator(world, t_max=20)
/// clauses = [c for t in range(gen.t_max + 1) for c in gen.generate(t)]
/// with Minisat22(bootstrap_with=clauses) as solver:
///     solver.append_formula(gen.objective(gen.t_max))
///     if solver.solve():
///         plan = gen.decode_plan(solver.get_model(), gen.t_max)
/// ```
#[gen_stub_pyclass]
#[pyclass(name = "ConstraintGenerator", module = "lle.solver.constraints")]
pub struct PyConstraintGenerator {
    inner: ConstraintGenerator,
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
impl PyConstraintGenerator {
    /// Build a constraint generator for `world`, considering plans of length up to `t_max`.
    #[new]
    fn new(world: &PyWorld, t_max: usize) -> Self {
        let inner = world.with_world(|world| ConstraintGenerator::new(world, t_max));
        let solution_lower_bound = inner.solution_lower_bound();
        PyConstraintGenerator {
            inner,
            t_max,
            solution_lower_bound,
        }
    }

    /// Generate every clause (initialization, movement and laser constraints) that applies
    /// at time step `t`. Clauses are returned as lists of signed integer literals (CNF),
    /// ready to be fed to a `pysat` solver.
    fn generate(&mut self, t: usize) -> Vec<Vec<i32>> {
        self.inner.generate(t)
    }

    /// Generate the objective clauses for time step `t`: every agent must be on an exit.
    /// Intended to be appended to the formula of the solver instance considering `t_end = t`.
    fn objective(&mut self, t: usize) -> Vec<Vec<i32>> {
        self.inner.objective(t)
    }

    /// Generate unit clauses forbidding any laser-blocking event at time `t`.
    ///
    /// Adding these clauses for every `t` in `[0, t_max]` is logically equivalent to
    /// forbidding cooperation altogether: the resulting formula is UNSAT iff laser
    /// blocking is required to solve the level within the horizon.
    fn no_blocking_clauses(&mut self, t: usize) -> Vec<Vec<i32>> {
        self.inner.no_blocking_clauses(t)
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
