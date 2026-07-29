use pyo3::{exceptions::PyValueError, prelude::*, types::PyAny};
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pymethods};

use super::pysolvemode::PySolveMode;
use crate::{
    bindings::{PyAction, PyWorld, pyexceptions::solver_error_to_exception},
    solver::{Clause, ClauseGenerator, Literal, SolveMode},
};

fn extract_solve_mode(py: Python, mode: Py<PyAny>) -> PyResult<SolveMode> {
    if let Ok(m) = mode.extract::<PySolveMode>(py) {
        Ok(m.into())
    } else if let Ok(s) = mode.extract::<String>(py) {
        s.parse::<SolveMode>().map_err(PyValueError::new_err)
    } else {
        Err(PyValueError::new_err(
            "mode must be a SolveMode enum or a string",
        ))
    }
}

/// Generates the SAT clauses (CNF, as lists of signed integer literals) used by
/// `lle.solver.Solver` and decodes solver models back into joint-action plans.
///
/// The constraint generation itself (agent movement, collisions, laser propagation and blocking)
/// is implemented in Rust for performance; SAT solving remains delegated to Python (e.g.
/// `pysat.solvers.Minisat22`). One generator can be reused across modes because domain clauses are
/// cached independently from cooperation-specific support clauses.
///
/// ```python
/// from pysat.solvers import Minisat22
/// from lle import World
/// from lle.solver.clauses import ClauseGenerator
///
/// world = World.level(1)
/// gen = ClauseGenerator(world, t_max=20)
/// clauses, assumptions = gen.generate(10, mode="standard", collect_gems=False)
/// with Minisat22(bootstrap_with=clauses) as solver:
///     if solver.solve(assumptions=assumptions):
///         plan = gen.decode_plan(solver.get_model(), 10)
/// ```
#[gen_stub_pyclass]
#[pyclass(name = "ClauseGenerator", module = "lle.solver.clauses")]
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
    #[new]
    fn new(world: &PyWorld, t_max: usize) -> PyResult<Self> {
        let inner = world.with_world(|world| ClauseGenerator::new(world, t_max));
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
    /// `mode` accepts either a `SolveMode` instance or its canonical string (`"standard"`,
    /// `"no-cooperation"`, `"no-asymmetric"`, `"no-mutual"`, `"no-chain[-N]"`,
    /// `"no-interdependence[-N]"`, `"no-convergence[-N]"`, `"no-divergence[-N]"`). `collect_gems` adds gem-collection
    /// clauses to the objective.
    ///
    /// Returns `(clauses, assumptions)` ready to be fed to `solve_model`.
    #[pyo3(signature = (t, mode=None, collect_gems=false))]
    fn generate(
        &mut self,
        py: Python,
        t: usize,
        #[gen_stub(override_type(
            type_repr = "typing.Literal['standard', 'no-cooperation', 'no-asymmetric', 'no-mutual', 'no-chain', 'no-interdependence', 'no-convergence', 'no-divergence'] | builtins.str | SolveMode | None",
            imports = ("typing",)
        ))]
        mode: Option<Py<PyAny>>,
        collect_gems: bool,
    ) -> PyResult<(Vec<Clause>, Vec<Literal>)> {
        let mode = match mode {
            Some(mode) => extract_solve_mode(py, mode)?,
            None => SolveMode::Standard,
        };
        Ok(self.inner.generate(t, mode, collect_gems))
    }

    /// Generate only the objective clauses for horizon `t`.
    ///
    /// Returns `(clauses, [])`. Useful for callers that manage the SAT solver directly and want to
    /// append the objective separately.
    #[pyo3(signature = (t, collect_gems=false))]
    fn objective(&mut self, t: usize, collect_gems: bool) -> (Vec<Clause>, Vec<Literal>) {
        (self.inner.objective(t, collect_gems), vec![])
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
