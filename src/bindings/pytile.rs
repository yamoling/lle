use std::sync::{Arc, Mutex};

use pyo3::prelude::*;

use crate::{
    agent::AgentId,
    tiles::{Direction, Gem, Laser, LaserId, LaserSource},
    Tile,
};

use super::pydirection::PyDirection;

#[pyclass(name = "Gem")]
pub struct PyGem {
    wrapped: Arc<Mutex<Gem>>,
}

impl From<Arc<Mutex<Gem>>> for PyGem {
    fn from(gem: Arc<Mutex<Gem>>) -> Self {
        PyGem { wrapped: gem }
    }
}

impl From<&Arc<Mutex<Gem>>> for PyGem {
    fn from(gem: &Arc<Mutex<Gem>>) -> Self {
        PyGem {
            wrapped: gem.clone(),
        }
    }
}

#[pymethods]
impl PyGem {
    pub fn __str__(&self) -> String {
        format!("{:?}", self.wrapped.lock().unwrap())
    }

    #[getter]
    pub fn agent(&self) -> Option<AgentId> {
        self.wrapped.lock().unwrap().agent()
    }

    #[getter]
    pub fn is_collected(&self) -> bool {
        self.wrapped.lock().unwrap().is_collected()
    }

    pub fn __repr__(&self) -> String {
        self.__str__()
    }
}

#[pyclass(name = "Laser")]
pub struct PyLaser {
    wrapped: Arc<Mutex<Laser>>,
}

#[pymethods]
impl PyLaser {
    #[getter]
    pub fn is_on(&self) -> bool {
        self.wrapped.lock().unwrap().is_on()
    }

    #[getter]
    pub fn is_off(&self) -> bool {
        !self.is_on()
    }

    #[getter]
    pub fn is_enabled(&self) -> bool {
        self.wrapped.lock().unwrap().is_enabled()
    }

    #[getter]
    pub fn is_disabled(&self) -> bool {
        !self.is_enabled()
    }

    #[getter]
    pub fn direction(&self) -> PyDirection {
        self.wrapped.lock().unwrap().direction().into()
    }

    #[getter]
    pub fn agent_id(&self) -> AgentId {
        self.wrapped.lock().unwrap().agent_id()
    }

    #[getter]
    pub fn agent(&self) -> Option<AgentId> {
        self.wrapped.lock().unwrap().agent()
    }

    #[getter]
    pub fn laser_id(&self) -> LaserId {
        self.wrapped.lock().unwrap().laser_id()
    }

    pub fn __str__(&self) -> String {
        let agent = match self.agent() {
            Some(agent) => agent.to_string(),
            None => String::from("None"),
        };

        format!(
            "Laser(laser_id={}, is_on={}, direction={}, agent_id={}, agent={agent})",
            self.laser_id(),
            self.is_on(),
            self.direction().name(),
            self.agent_id()
        )
    }

    pub fn __repr__(&self) -> String {
        self.__str__()
    }
}

// Implement from and into
impl From<&Arc<Mutex<Laser>>> for PyLaser {
    fn from(laser: &Arc<Mutex<Laser>>) -> Self {
        PyLaser {
            wrapped: laser.clone(),
        }
    }
}

#[pyclass(name = "LaserSource", module = "lle")]
#[derive(Debug)]
pub struct PyLaserSource {
    direction: PyDirection,
    agent_id: AgentId,
    laser_id: LaserId,
    is_enabled: bool,
}

impl From<&Arc<Mutex<LaserSource>>> for PyLaserSource {
    fn from(source: &Arc<Mutex<LaserSource>>) -> Self {
        PyLaserSource {
            direction: source.direction().into(),
            agent_id: source.agent_id(),
            laser_id: source.laser_id(),
            is_enabled: source.is_enabled(),
        }
    }
}

#[pymethods]
impl PyLaserSource {
    #[new]
    /// This constructor is required for pickling but should not be used for any other purpose.
    fn new() -> Self {
        PyLaserSource {
            direction: Direction::North.into(),
            agent_id: 0,
            laser_id: 0,
            is_enabled: false,
        }
    }

    #[getter]
    pub fn is_enabled(&self) -> bool {
        self.is_enabled
    }

    #[getter]
    pub fn is_disabled(&self) -> bool {
        !self.is_enabled
    }

    #[getter]
    pub fn direction(&self) -> PyDirection {
        self.direction.clone()
    }

    #[getter]
    pub fn agent_id(&self) -> AgentId {
        self.agent_id
    }

    #[getter]
    pub fn laser_id(&self) -> LaserId {
        self.laser_id
    }

    pub fn __str__(&self) -> String {
        format!("{:?}", self)
    }

    pub fn __repr__(&self) -> String {
        self.__str__()
    }

    pub fn __getstate__(&self) -> (PyDirection, AgentId, LaserId, bool) {
        (
            self.direction.clone(),
            self.agent_id,
            self.laser_id,
            self.is_enabled,
        )
    }

    pub fn __setstate__(&mut self, state: (PyDirection, AgentId, LaserId, bool)) {
        self.direction = state.0;
        self.agent_id = state.1;
        self.laser_id = state.2;
        self.is_enabled = state.3;
    }
}
