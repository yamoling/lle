use pyo3::{exceptions::PyValueError, prelude::*};
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pymethods};

use crate::solver::SolveMode;

/// The solving mode used by `ClauseGenerator`.
///
/// Build one with the factory methods (`SolveMode.standard()`, `SolveMode.no_chain(length=3)`,
/// …) or parse one from its canonical string with `SolveMode.from_str("no-chain-3")`. The
/// available modes control which extra constraints and assumptions are emitted by `generate(t)`:
///
/// - `standard()` — world rules only; agents may cooperate freely.
/// - `no_cooperation()` — forbids any non-owner agent from occupying a laser span. Equivalent to
///   treating every beam as permanently active.
/// - `no_asymmetric()` — rules out plans where an agent helps someone without ever being helped.
/// - `no_mutual()` — rules out plans where two agents each help the other.
/// - `no_chain(length=2)` — rules out plans containing a temporal chain of `length` help edges or
///   more (`a → b → c` is a chain of length 2). Subsumes `no_mutual()` (a mutual cycle is a chain
///   of length 2).
/// - `no_interdependence(order=2)` — rules out plans whose dependency graph contains a temporal
///   cycle visiting `order` distinct agents or more. For two-agent worlds this coincides with
///   `no_mutual()`.
///
/// ```python
/// from lle.solver.constraints import ClauseGenerator, SolveMode
/// from lle import World
///
/// gen = ClauseGenerator(World.level(6), t_max=21, mode=SolveMode.no_mutual())
/// for t in range(gen.solution_lower_bound, gen.t_max + 1):
///     clauses, assumptions = gen.generate(t)
///     ...
/// ```
#[gen_stub_pyclass]
#[pyclass(
    name = "SolveMode",
    module = "lle.solver.constraints",
    frozen,
    eq,
    hash,
    from_py_object
)]
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct PySolveMode {
    inner: SolveMode,
}

impl From<&PySolveMode> for SolveMode {
    fn from(m: &PySolveMode) -> Self {
        m.inner
    }
}

impl From<PySolveMode> for SolveMode {
    fn from(m: PySolveMode) -> Self {
        m.inner
    }
}

impl From<SolveMode> for PySolveMode {
    fn from(inner: SolveMode) -> Self {
        Self { inner }
    }
}

#[gen_stub_pymethods]
#[pymethods]
impl PySolveMode {
    /// World rules only; agents may cooperate freely.
    #[staticmethod]
    fn standard() -> Self {
        SolveMode::Standard.into()
    }

    /// Forbid any non-owner agent from entering a laser span (every beam is treated as active).
    #[staticmethod]
    fn no_cooperation() -> Self {
        SolveMode::NoCooperation.into()
    }

    /// Forbid plans where an agent helps someone without ever being helped by another agent.
    #[staticmethod]
    fn no_asymmetric() -> Self {
        SolveMode::NoAsymmetricCooperation.into()
    }

    /// Forbid plans where two agents each help the other.
    #[staticmethod]
    fn no_mutual() -> Self {
        SolveMode::NoMutualCooperation.into()
    }

    /// Forbid any temporal chain of `length` help edges or more. `length` must be `>= 2`.
    #[staticmethod]
    #[pyo3(signature = (length=2))]
    fn no_chain(length: usize) -> PyResult<Self> {
        Self::checked(length, "no_chain", SolveMode::NoChainedCooperation)
    }

    /// Forbid any temporal cycle visiting `order` distinct agents or more. `order` must be `>= 2`.
    #[staticmethod]
    #[pyo3(signature = (order=2))]
    fn no_interdependence(order: usize) -> PyResult<Self> {
        Self::checked(order, "no_interdependence", SolveMode::NoInterdependence)
    }

    /// Parse a canonical string (e.g. `"standard"`, `"no-chain-3"`, `"no-interdependence-2"`).
    #[staticmethod]
    pub fn from_str(value: &str) -> PyResult<Self> {
        SolveMode::from_str(value)
            .map(Into::into)
            .map_err(PyValueError::new_err)
    }

    /// The canonical string representation, inverse of `from_str` (e.g. `"no-chain-3"`).
    /// The default length is rendered without a suffix (`"no-chain"`, `"no-interdependence"`).
    #[getter]
    pub fn value(&self) -> String {
        self.inner.canonical()
    }

    /// The base modes, each with their default length. Parametrized modes with a non-default
    /// length must be built explicitly via `no_chain(...)` / `no_interdependence(...)`.
    #[staticmethod]
    fn variants() -> Vec<PySolveMode> {
        [
            SolveMode::Standard,
            SolveMode::NoCooperation,
            SolveMode::NoAsymmetricCooperation,
            SolveMode::NoMutualCooperation,
            SolveMode::NoChainedCooperation(2),
            SolveMode::NoInterdependence(2),
        ]
        .into_iter()
        .map(Into::into)
        .collect()
    }

    fn __str__(&self) -> String {
        self.inner.canonical()
    }

    fn __repr__(&self) -> String {
        format!("SolveMode.from_str({:?})", self.inner.canonical())
    }
}

impl PySolveMode {
    /// Build a parametrized mode, rejecting lengths below the minimum (`2`).
    fn checked(n: usize, factory: &str, build: fn(usize) -> SolveMode) -> PyResult<Self> {
        if n < 2 {
            return Err(PyValueError::new_err(format!(
                "{factory}: the minimal rejected length must be >= 2, got {n}."
            )));
        }
        Ok(build(n).into())
    }
}
