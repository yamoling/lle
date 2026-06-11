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
}

impl From<&PySolveMode> for SolveMode {
    fn from(m: &PySolveMode) -> Self {
        match m {
            PySolveMode::Standard => SolveMode::Standard,
            PySolveMode::NoCooperation => SolveMode::NoCooperation,
            PySolveMode::NoMutualCooperation => SolveMode::NoMutualCooperation,
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
    const fn variants() -> [PySolveMode; 3] {
        [
            PySolveMode::Standard,
            PySolveMode::NoCooperation,
            PySolveMode::NoMutualCooperation,
        ]
    }

    /// The canonical string representation, e.g. `"no-cooperation"`.
    /// Matches the string literals accepted by `ClauseGenerator` and `solve`.
    #[getter]
    #[gen_stub(override_return_type(type_repr="typing.Literal['standard', 'no-cooperation', 'no-mutual-cooperation']", imports=("typing")))]
    pub fn value(&self) -> &'static str {
        match self {
            Self::Standard => "standard",
            Self::NoCooperation => "no-cooperation",
            Self::NoMutualCooperation => "no-mutual-cooperation",
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
            }
        )
    }

    fn __hash__(&self) -> u64 {
        match self {
            Self::Standard => 0,
            Self::NoCooperation => 1,
            Self::NoMutualCooperation => 2,
        }
    }

    #[staticmethod]
    pub fn from_str(
        #[gen_stub(override_type(type_repr="typing.Literal['standard', 'no-cooperation', 'no-mutual-cooperation']", imports=("typing")))]
        value: &str,
    ) -> PyResult<Self> {
        match value {
            "standard" => Ok(Self::Standard),
            "no-cooperation" => Ok(Self::NoCooperation),
            "no-mutual-cooperation" => Ok(Self::NoMutualCooperation),
            _ => Err(PyValueError::new_err(format!(
                "invalid solve mode: {value}"
            ))),
        }
    }
}
