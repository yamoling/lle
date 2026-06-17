use pyo3::{exceptions::PyValueError, prelude::*, types::PyAny};
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pymethods};

use super::pysolvemode::PySolveMode;
use crate::{
    bindings::{PyAction, PyWorld, pyexceptions::solver_error_to_exception},
    solver::{Clause, ClauseGenerator, Literal, SolveMode},
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
/// - `"no-asymmetric"`: clauses characterizing asymmetric cooperation and assumptions that such cooperation is forbidden.
/// - `"no-mutual"`: clauses and assumptions forbidding pairs of agents from
///   mutually helping each other.
/// - `"no-chain"` / `"no-chain-N"`: forbid any non-decreasing-time temporal chain of
///   `N` help edges or more (`a → b → c` is a chain of length 2). `N` defaults to 2.
/// - `"no-interdependence"` / `"no-interdependence-N"`: forbid any temporal cycle visiting `N`
///   distinct agents or more. `N` defaults to 2, which coincides with `"no-mutual"` for two agents.
///
/// ```python
/// from pysat.solvers import Minisat22
/// from lle import World
/// from lle.solver.constraints import ClauseGenerator
///
/// world = World.level(1)
/// gen = ClauseGenerator(world, t_max=20, mode="standard")
/// clauses, assumptions = gen.generate(t_max)
/// with Minisat22(bootstrap_with=clauses) as solver:
///     if solver.solve(assumptions=assumptions):
///         plan = gen.decode_plan(solver.get_model(), t)
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
    /// `mode` selects the solving strategy. It accepts either a `SolveMode` instance or its
    /// canonical string (`"standard"`, `"no-cooperation"`, `"no-asymmetric"`, `"no-mutual"`,
    /// `"no-chain[-N]"`, `"no-interdependence[-N]"`). Defaults to `"standard"`.
    #[new]
    fn new(
        py: Python,
        world: &PyWorld,
        t_max: usize,
        #[gen_stub(override_type(
            type_repr = "typing.Literal['standard', 'no-cooperation', 'no-asymmetric', 'no-mutual', 'no-chain', 'no-interdependence'] | SolveMode",
            imports = ("typing",)
        ))]
        mode: Py<PyAny>,
    ) -> PyResult<Self> {
        let mode: SolveMode = if let Ok(m) = mode.extract::<PySolveMode>(py) {
            m.into()
        } else if let Ok(s) = mode.extract::<String>(py) {
            SolveMode::from_str(&s).map_err(PyValueError::new_err)?
        } else {
            return Err(PyValueError::new_err(
                "mode must be a SolveMode enum or a string",
            ));
        };
        let inner = world.with_world(|world| ClauseGenerator::new(world, t_max, mode));
        let solution_lower_bound = inner.solution_lower_bound();
        Ok(Self {
            inner,
            t_max,
            solution_lower_bound,
        })
    }

    /// The number of SAT variables allocated so far by this generator.
    #[getter]
    fn n_vars(&self) -> usize {
        self.inner.n_vars()
    }

    /// Generate all clauses and assumptions required to solve the problem at horizon `t`.
    ///
    /// Buffers world-enforcing clauses incrementally (each step is computed at most once),
    /// then returns the full formula for this horizon:
    /// - All buffered clauses for steps `0..=t`
    /// - The objective clauses (every agent on an exit at step `t`)
    /// - For `"no-asymmetric"` mode: asymmetric-forbid clauses and assumptions
    /// - For `"no-mutual"` mode: mutual-forbid clauses and assumptions
    /// - For `"no-chain"` mode: chain-forbid assumptions
    /// - For `"no-interdependence"` mode: cycle-forbid assumptions
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
            Err(e) => Err(solver_error_to_exception(e)),
        }
    }
}
