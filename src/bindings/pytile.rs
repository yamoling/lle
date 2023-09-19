use pyo3::prelude::*;

use crate::{
    agent::AgentId,
    tiles::{LaserSource, Tile},
};

use super::pydirection::PyDirection;

#[pyclass(name = "Gem")]
pub struct PyGem {
    #[pyo3(get, set)]
    agent: Option<AgentId>,
    #[pyo3(get, set)]
    is_collected: bool,
}

impl PyGem {
    pub fn new(agent: Option<AgentId>, is_collected: bool) -> Self {
        PyGem {
            agent,
            is_collected,
        }
    }
}

#[pyclass(name = "Laser")]
pub struct PyLaser {
    #[pyo3(get, set)]
    is_on: bool,
    #[pyo3(get, set)]
    direction: PyDirection,
    #[pyo3(get, set)]
    agent_id: AgentId,
    #[pyo3(get, set)]
    agent: Option<AgentId>,
}

impl PyLaser {
    pub fn new(
        is_on: bool,
        direction: PyDirection,
        agent_id: AgentId,
        agent: Option<AgentId>,
    ) -> Self {
        PyLaser {
            is_on,
            direction,
            agent,
            agent_id,
        }
    }
}

#[pymethods]
impl PyLaser {
    #[getter]
    fn is_off(&self) -> bool {
        !self.is_on
    }
}

#[pyclass(name = "LaserSource")]
pub struct PyLaserSource {
    source: LaserSource,
}

impl PyLaserSource {
    pub fn new(source: LaserSource) -> Self {
        PyLaserSource { source }
    }
}

#[pymethods]
impl PyLaserSource {
    #[getter]
    fn direction(&self) -> PyDirection {
        PyDirection::new(self.source.direction())
    }

    #[getter]
    fn agent_id(&self) -> AgentId {
        self.source.agent_id()
    }

    #[getter]
    fn agent(&self) -> Option<AgentId> {
        self.source.agent()
    }
}
