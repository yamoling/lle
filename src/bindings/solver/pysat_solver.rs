use pyo3::prelude::*;
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pymethods};

use crate::{
    bindings::pyexceptions::solver_error_to_exception,
    solver::{Clause, Literal, SatSolver},
};

/// An incremental SAT solver (CaDiCaL) for the clauses of a `ClauseGenerator`.
///
/// Clauses are permanent once added, while assumptions only hold for one call to `solve`.
///
/// ```python
/// from lle import World
/// from lle.solver.clauses import ClauseGenerator, SatSolver
///
/// gen = ClauseGenerator(World.level(1), t_max=20)
/// clauses, assumptions = gen.generate(10, mode="standard", collect_gems=False)
/// solver = SatSolver()
/// solver.append_formula(clauses)
/// if solver.solve(assumptions):
///     plan = gen.decode_plan(solver.get_model(), 10)
/// ```
#[gen_stub_pyclass]
#[pyclass(name = "SatSolver", module = "lle.solver.clauses", unsendable)]
pub struct PySatSolver {
    inner: SatSolver,
}

#[gen_stub_pymethods]
#[pymethods]
impl PySatSolver {
    #[new]
    fn new() -> Self {
        Self {
            inner: SatSolver::new(),
        }
    }

    /// The largest variable seen so far in a clause or an assumption.
    #[getter]
    fn n_vars(&self) -> usize {
        self.inner.n_vars()
    }

    /// Add one clause, as a list of non-zero signed literals.
    ///
    /// # Raises
    /// - `SolverError`: if a literal is 0.
    fn add_clause(&mut self, clause: Clause) -> PyResult<()> {
        self.inner
            .add_clause(&clause)
            .map_err(solver_error_to_exception)
    }

    /// Add every clause of `clauses`.
    ///
    /// # Raises
    /// - `SolverError`: if a literal is 0.
    fn append_formula(&mut self, clauses: Vec<Clause>) -> PyResult<()> {
        self.inner
            .add_clauses(&clauses)
            .map_err(solver_error_to_exception)
    }

    /// Solve the clauses added so far under `assumptions`, and return whether they are
    /// satisfiable.
    ///
    /// # Raises
    /// - `SolverError`: if an assumption is 0.
    #[pyo3(signature = (assumptions=Vec::new()))]
    fn solve(&mut self, assumptions: Vec<Literal>) -> PyResult<bool> {
        self.inner
            .solve(&assumptions)
            .map_err(solver_error_to_exception)
    }

    /// The model found by the last call to `solve`, as one signed literal per variable from 1 to
    /// `n_vars`, or `None` if that call was unsatisfiable or a clause was added since.
    fn get_model(&self) -> Option<Vec<Literal>> {
        self.inner.model()
    }
}
