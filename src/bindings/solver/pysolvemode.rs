use pyo3::{exceptions::PyValueError, prelude::*};
use pyo3_stub_gen::derive::{gen_stub_pyclass_enum, gen_stub_pymethods};

use crate::solver::SolveMode;

/// The solving mode used by `ClauseGenerator`.
///
/// Controls which extra constraints and assumptions are emitted by `generate(t)`:
///
/// - `STANDARD` — world rules only; agents may cooperate freely.
/// - `NO_COOPERATION` — per-step unit assumptions forbidding any non-owner agent from occupying
///   a laser span. Equivalent to treating every beam as permanently active.
/// - `NO_MUTUAL_COOPERATION` — dependency indicator clauses plus horizon-level mutual-forbid
///   clauses and assumptions, ruling out plans where two agents each help the other.
/// - `NO_CHAINED_COOPERATION` — chain indicator clauses plus horizon-level chain-forbid
///   assumptions, ruling out plans where a temporal chain `a → b → c` exists (a helped b, then
///   b helped c). Subsumes `NO_MUTUAL_COOPERATION` (cycles are a special case of chains).
/// - `NO_INTERDEPENDENCE` — walk indicator clauses plus horizon-level forbid assumptions, ruling
///   out plans whose dependency graph contains any temporal cycle (a closed help chain that
///   returns to its start with non-decreasing timestamps).
///
/// ```python
/// from lle.solver.constraints import ClauseGenerator, SolveMode
/// from lle import World
///
/// gen = ClauseGenerator(World.level(6), t_max=21, mode=SolveMode.NO_MUTUAL_COOPERATION)
/// for t in range(gen.solution_lower_bound, gen.t_max + 1):
///     clauses, assumptions = gen.generate(t)
///     ...
/// ```
#[gen_stub_pyclass_enum]
#[pyclass(
    name = "SolveMode",
    module = "lle.solver.constraints",
    eq,
    frozen,
    from_py_object
)]
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum PySolveMode {
    #[pyo3(name = "STANDARD")]
    Standard,
    #[pyo3(name = "NO_COOPERATION")]
    NoCooperation,
    #[pyo3(name = "NO_MUTUAL_COOPERATION")]
    NoMutualCooperation,
    #[pyo3(name = "NO_CHAINED_COOPERATION")]
    NoChainedCooperation,
    #[pyo3(name = "NO_INTERDEPENDENCE")]
    NoInterdependence,
}

impl From<&PySolveMode> for SolveMode {
    fn from(m: &PySolveMode) -> Self {
        match m {
            PySolveMode::Standard => SolveMode::Standard,
            PySolveMode::NoCooperation => SolveMode::NoCooperation,
            PySolveMode::NoMutualCooperation => SolveMode::NoMutualCooperation,
            PySolveMode::NoChainedCooperation => SolveMode::NoChainedCooperation,
            PySolveMode::NoInterdependence => SolveMode::NoInterdependence,
        }
    }
}

impl From<PySolveMode> for SolveMode {
    fn from(m: PySolveMode) -> Self {
        SolveMode::from(&m)
    }
}

#[gen_stub_pymethods]
#[pymethods]
impl PySolveMode {
    #[staticmethod]
    const fn variants() -> [PySolveMode; 5] {
        [
            PySolveMode::Standard,
            PySolveMode::NoCooperation,
            PySolveMode::NoMutualCooperation,
            PySolveMode::NoChainedCooperation,
            PySolveMode::NoInterdependence,
        ]
    }

    /// The canonical string representation, e.g. `"no-cooperation"`.
    /// Matches the string literals accepted by `ClauseGenerator` and `solve`.
    #[getter]
    #[gen_stub(override_return_type(type_repr="typing.Literal['standard', 'no-cooperation', 'no-mutual-cooperation', 'no-chained-cooperation', 'no-interdependence']", imports=("typing")))]
    pub fn value(&self) -> &'static str {
        match self {
            Self::Standard => "standard",
            Self::NoCooperation => "no-cooperation",
            Self::NoMutualCooperation => "no-mutual-cooperation",
            Self::NoChainedCooperation => "no-chained-cooperation",
            Self::NoInterdependence => "no-interdependence",
        }
    }

    fn __str__(&self) -> &'static str {
        self.value()
    }

    fn __repr__(&self) -> String {
        format!(
            "SolveMode.{}",
            match self {
                Self::Standard => "STANDARD",
                Self::NoCooperation => "NO_COOPERATION",
                Self::NoMutualCooperation => "NO_MUTUAL_COOPERATION",
                Self::NoChainedCooperation => "NO_CHAINED_COOPERATION",
                Self::NoInterdependence => "NO_INTERDEPENDENCE",
            }
        )
    }

    fn __hash__(&self) -> u64 {
        match self {
            Self::Standard => 0,
            Self::NoCooperation => 1,
            Self::NoMutualCooperation => 2,
            Self::NoChainedCooperation => 3,
            Self::NoInterdependence => 4,
        }
    }

    #[staticmethod]
    pub fn from_str(
        #[gen_stub(override_type(type_repr="typing.Literal['standard', 'no-cooperation', 'no-mutual-cooperation', 'no-chained-cooperation', 'no-interdependence']", imports=("typing")))]
        value: &str,
    ) -> PyResult<Self> {
        match value {
            "standard" => Ok(Self::Standard),
            "no-cooperation" => Ok(Self::NoCooperation),
            "no-mutual-cooperation" => Ok(Self::NoMutualCooperation),
            "no-chained-cooperation" => Ok(Self::NoChainedCooperation),
            "no-interdependence" => Ok(Self::NoInterdependence),
            _ => Err(PyValueError::new_err(format!(
                "invalid solve mode: {value}"
            ))),
        }
    }
}
