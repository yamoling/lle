use pyo3::{prelude::*, types::PyAny};
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pymethods};

use super::pyclause_generator::extract_solve_mode;
use crate::{
    bindings::{PyAction, PyWorld, pyexceptions::solver_error_to_exception},
    solver::{Plan, SolveMode, Solver},
};

fn to_py_plan(plan: Option<Plan>) -> Option<Vec<Vec<PyAction>>> {
    plan.map(|plan| {
        plan.into_iter()
            .map(|joint| joint.iter().map(PyAction::from).collect())
            .collect()
    })
}

/// The Rust core of `lle.solver.Solver`: generates the clauses and solves them with CaDiCaL
/// without handing them to Python.
///
/// Arguments are not validated here; use `lle.solver.Solver` instead.
#[gen_stub_pyclass]
#[pyclass(name = "Solver", module = "lle.solver.clauses")]
pub struct PySolver {
    inner: Solver,
}

#[gen_stub_pymethods]
#[pymethods]
impl PySolver {
    /// Build a solver for `world`, considering plans of length up to `t_max`.
    ///
    /// # Raises
    /// - `ValueError`: if two agents share a colour.
    #[new]
    fn new(world: &PyWorld, t_max: usize) -> PyResult<Self> {
        let inner = world
            .with_world(|world| Solver::new(world, t_max))
            .map_err(solver_error_to_exception)?;
        Ok(Self { inner })
    }

    /// The maximum plan length considered by this solver.
    #[getter]
    fn t_max(&self) -> usize {
        self.inner.t_max()
    }

    /// A cheap admissible lower bound on the length of any valid plan.
    #[getter]
    fn solution_lower_bound(&self) -> usize {
        self.inner.solution_lower_bound()
    }

    /// Find a plan of exactly `path_length` joint actions, or `None` if there is none.
    ///
    /// # Raises
    /// - `ValueError`: if `mode` is invalid.
    #[pyo3(signature = (path_length, mode=None, collect_gems=false, shuffle=false))]
    fn solve(
        &mut self,
        py: Python,
        path_length: usize,
        #[gen_stub(override_type(
            type_repr = "typing.Literal['standard', 'no-cooperation', 'no-asymmetric', 'no-mutual', 'no-fully-coupled', 'no-sequence', 'no-interdependence', 'no-convergence', 'no-divergence'] | builtins.str | SolveMode | None",
            imports = ("typing",)
        ))]
        mode: Option<Py<PyAny>>,
        collect_gems: bool,
        shuffle: bool,
    ) -> PyResult<Option<Vec<Vec<PyAction>>>> {
        let mode = match mode {
            Some(mode) => extract_solve_mode(py, mode)?,
            None => SolveMode::Standard,
        };
        let inner = &mut self.inner;
        let plan = py
            .detach(|| inner.solve(path_length, mode, collect_gems, shuffle))
            .map_err(solver_error_to_exception)?;
        Ok(to_py_plan(plan))
    }

    /// Find the shortest plan from `t_min` (at least `solution_lower_bound`) through `t_max`, or
    /// `None` if there is none.
    ///
    /// # Raises
    /// - `ValueError`: if `mode` is invalid.
    #[pyo3(signature = (mode=None, t_min=None, collect_gems=false, shuffle=false))]
    fn find_shortest(
        &mut self,
        py: Python,
        #[gen_stub(override_type(
            type_repr = "typing.Literal['standard', 'no-cooperation', 'no-asymmetric', 'no-mutual', 'no-fully-coupled', 'no-sequence', 'no-interdependence', 'no-convergence', 'no-divergence'] | builtins.str | SolveMode | None",
            imports = ("typing",)
        ))]
        mode: Option<Py<PyAny>>,
        t_min: Option<usize>,
        collect_gems: bool,
        shuffle: bool,
    ) -> PyResult<Option<Vec<Vec<PyAction>>>> {
        let mode = match mode {
            Some(mode) => extract_solve_mode(py, mode)?,
            None => SolveMode::Standard,
        };
        let inner = &mut self.inner;
        let plan = py
            .detach(|| inner.find_shortest(mode, t_min, collect_gems, shuffle))
            .map_err(solver_error_to_exception)?;
        Ok(to_py_plan(plan))
    }
}
