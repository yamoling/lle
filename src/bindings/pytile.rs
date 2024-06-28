use std::sync::{Arc, Mutex};

use pyo3::prelude::*;

use crate::{
    agent::AgentId,
    tiles::{Gem, Laser, LaserId, LaserSource},
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
    wrapped: Arc<Mutex<LaserSource>>,
}

impl From<&Arc<Mutex<LaserSource>>> for PyLaserSource {
    fn from(source: &Arc<Mutex<LaserSource>>) -> Self {
        PyLaserSource {
            wrapped: source.clone(),
        }
    }
}

#[pymethods]
impl PyLaserSource {
    #[getter]
    pub fn is_enabled(&self) -> bool {
        self.wrapped.lock().unwrap().is_enabled()
    }

    #[getter]
    pub fn is_disabled(&self) -> bool {
        !self.is_enabled()
    }

    pub fn disable(&self) {
        self.wrapped.lock().unwrap().disable();
    }

    pub fn enable(&self) {
        self.wrapped.lock().unwrap().enable();
    }

    pub fn set_color(&self, agent_id: i32) -> PyResult<()> {
        self.set_colour(agent_id)
    }

    pub fn set_colour(&self, agent_id: i32) -> PyResult<()> {
        if agent_id < 0 {
            return Err(pyo3::exceptions::PyValueError::new_err(
                "Agent ID must be positive",
            ));
        }
        Ok(self
            .wrapped
            .lock()
            .unwrap()
            .set_agent_id(agent_id as AgentId))
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
    pub fn laser_id(&self) -> LaserId {
        self.wrapped.lock().unwrap().laser_id()
    }

    pub fn __str__(&self) -> String {
        format!("{:?}", self)
    }

    pub fn __repr__(&self) -> String {
        self.__str__()
    }
}
