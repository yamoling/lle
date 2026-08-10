use pyo3::{exceptions::PyValueError, prelude::*};
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pymethods};

use crate::solver::SolveMode;

/// The solving mode used by `ClauseGenerator`.
///
/// Build one with the factory methods (`SolveMode.standard()`, `SolveMode.no_chain(length=3)`,
/// …) or parse one from its canonical string with `SolveMode.from_str("no-chain-3")`. The
/// available modes control which extra clauses and assumptions are emitted by `generate(t)`:
///
/// - `standard()` — world rules only; agents may cooperate freely.
/// - `no_cooperation()` — forbids any non-owner agent from occupying a laser span. Equivalent to
///   treating every beam as permanently active.
/// - `no_asymmetric()` — rules out plans where an agent helps someone without ever being helped.
/// - `no_mutual()` — rules out plans where two agents each help the other.
/// - `no_fully_coupled()` — requires at least one ordered pair of agents to lack help.
/// - `no_chain(length=2)` — rules out plans containing a non-decreasing-time temporal chain of
///   `length` help edges or more (`a → b → c` is a chain of length 2).
/// - `no_interdependence(order=2)` — rules out plans whose dependency graph contains a temporal
///   closed trail with exactly `order` distinct agents. Timestamps are non-decreasing; agents and
///   static arcs may repeat at later times, but temporal edges may not repeat. Other exact orders
///   remain allowed. `no_interdependence(2)` coincides with `no_mutual()`.
/// - `no_convergence(k=2)` — rules out plans where one beneficiary receives help from at least `k`
///   distinct helpers.
/// - `no_divergence(k=2)` — rules out plans where one helper helps at least `k` distinct
///   beneficiaries. This is the outgoing dual of `no_convergence`.
///
/// ```python
/// from lle.solver.clauses import ClauseGenerator, SolveMode
/// from lle import World
///
/// gen = ClauseGenerator(World.level(6), t_max=21)
/// for t in range(gen.solution_lower_bound, gen.t_max + 1):
///     clauses, assumptions = gen.generate(t, mode=SolveMode.no_chain(2))
///     ...
/// ```
#[gen_stub_pyclass]
#[pyclass(
    name = "SolveMode",
    module = "lle.solver.clauses",
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

impl std::str::FromStr for PySolveMode {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, String> {
        s.parse::<SolveMode>().map(Into::into)
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
    ///
    /// Equivalent to [`SolveMode::NoInterdependence(2)`].
    #[staticmethod]
    fn no_mutual() -> Self {
        SolveMode::NoInterdependence(2).into()
    }

    /// Require at least one ordered pair of distinct agents to lack a help event.
    #[staticmethod]
    fn no_fully_coupled() -> Self {
        SolveMode::NoFullyCoupledCooperation.into()
    }

    /// Forbid any non-decreasing-time temporal chain of `length` help edges or more. `length` must be `>= 2`.
    #[staticmethod]
    #[pyo3(signature = (length=2))]
    fn no_chain(length: i64) -> PyResult<Self> {
        Self::checked(length, "no_chain", SolveMode::NoChainedCooperation)
    }

    /// Forbid any temporal closed trail with exactly `order` distinct agents. `order` must be `>= 2`.
    #[staticmethod]
    #[pyo3(signature = (order=2))]
    fn no_interdependence(order: i64) -> PyResult<Self> {
        Self::checked(order, "no_interdependence", SolveMode::NoInterdependence)
    }

    /// Forbid any beneficiary from receiving help from at least `k` distinct helpers. `k` must be `>= 2`.
    #[staticmethod]
    #[pyo3(signature = (k=2))]
    fn no_convergence(k: i64) -> PyResult<Self> {
        Self::checked(k, "no_convergence", SolveMode::NoConvergentCooperation)
    }

    /// Forbid any helper from helping at least `k` distinct beneficiaries. `k` must be `>= 2`.
    ///
    /// @ai-generated
    #[staticmethod]
    #[pyo3(signature = (k=2))]
    fn no_divergence(k: i64) -> PyResult<Self> {
        Self::checked(k, "no_divergence", SolveMode::NoDivergentCooperation)
    }

    /// Parse a canonical string (e.g. `"standard"`, `"no-chain-3"`, `"no-divergence-3"`).
    ///
    /// `"no-chain"`, `"no-interdependence"`, `"no-convergence"`, and `"no-divergence"` accept a
    /// `"-n"` suffix for their parameter. Their bare forms are aliases for the corresponding
    /// `"-2"` forms.
    ///
    /// @ai-generated
    #[staticmethod]
    #[pyo3(name = "from_str")]
    pub fn parse(
        #[gen_stub(override_type(
            type_repr = "typing.Literal['standard', 'no-cooperation', 'no-asymmetric', 'no-mutual', 'no-fully-coupled', 'no-chain', 'no-interdependence', 'no-convergence', 'no-divergence'] | builtins.str"
        ))]
        value: &str,
    ) -> PyResult<Self> {
        value.parse::<Self>().map_err(PyValueError::new_err)
    }

    /// The canonical string representation, inverse of `from_str` (e.g. `"no-chain-3"`).
    /// Default parameters are rendered without a suffix (`"no-chain"`, `"no-interdependence"`,
    /// `"no-convergence"`, `"no-divergence"`).
    #[getter]
    pub fn value(&self) -> String {
        self.inner.canonical()
    }

    fn __str__(&self) -> String {
        self.inner.canonical()
    }

    fn __repr__(&self) -> String {
        format!("SolveMode.from_str({:?})", self.inner.canonical())
    }
}

impl PySolveMode {
    /// Build a parametrized mode, rejecting thresholds below the minimum (`2`).
    ///
    /// The parameter is accepted as a signed integer so that negative Python values reach this
    /// explicit check instead of failing earlier with an `OverflowError` during conversion.
    ///
    /// @ai-generated
    fn checked(n: i64, factory: &str, build: fn(usize) -> SolveMode) -> PyResult<Self> {
        if n < 2 {
            return Err(PyValueError::new_err(format!(
                "{factory}: the minimal rejected threshold must be >= 2, got {n}."
            )));
        }
        Ok(build(n as usize).into())
    }
}
