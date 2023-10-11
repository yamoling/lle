use crate::{world::WorldState, Position};
use pyo3::{exceptions, prelude::*, pyclass::CompareOp, types::PyDict};
use std::hash::{Hash, Hasher};

#[pyclass(name = "WorldState", module = "lle")]
#[derive(Clone, Hash)]
pub struct PyWorldState {
    #[pyo3(get, set)]
    /// The position of each agent.
    agents_positions: Vec<Position>,
    #[pyo3(get, set)]
    /// The collection status of each gem.
    gems_collected: Vec<bool>,
}

#[pymethods]
impl PyWorldState {
    #[new]
    /// Construct a WorldState from the position of each agent and the collection status of each gem.
    pub fn new(agents_positions: Vec<Position>, gems_collected: Vec<bool>) -> Self {
        Self {
            agents_positions,
            gems_collected,
        }
    }

    fn __deepcopy__(&self, _memo: &PyDict) -> Self {
        self.clone()
    }

    fn __getstate__(&self) -> PyResult<(Vec<bool>, Vec<Position>)> {
        Ok((self.gems_collected.clone(), self.agents_positions.clone()))
    }

    fn __setstate__(&mut self, state: (Vec<bool>, Vec<Position>)) -> PyResult<()> {
        let (gems_collected, agents_positions) = state;
        self.gems_collected = gems_collected;
        self.agents_positions = agents_positions;
        Ok(())
    }

    pub fn __getnewargs__(&self) -> PyResult<(Vec<Position>, Vec<bool>)> {
        Ok((vec![], vec![]))
    }

    fn __str__(&self) -> String {
        format!(
            "WorldState(agent_positions={:?}, gems_collected={:?})",
            self.agents_positions, self.gems_collected
        )
    }

    fn __repr__(&self) -> String {
        self.__str__()
    }

    fn __hash__(&self) -> u64 {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        self.hash(&mut hasher);
        hasher.finish()
    }

    fn __richcmp__(&self, other: &Self, cmp: CompareOp) -> PyResult<bool> {
        let eq = self.agents_positions == other.agents_positions
            && self.gems_collected == other.gems_collected;
        match cmp {
            CompareOp::Eq => Ok(eq),
            CompareOp::Ne => Ok(!eq),
            other => Err(exceptions::PyArithmeticError::new_err(format!(
                "Unsupported comparison: {other:?}"
            ))),
        }
    }
}

impl From<PyWorldState> for WorldState {
    fn from(val: PyWorldState) -> Self {
        WorldState {
            agents_positions: val.agents_positions,
            gems_collected: val.gems_collected,
        }
    }
}
