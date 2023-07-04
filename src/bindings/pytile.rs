use pyo3::prelude::*;

use crate::{
    agent::AgentId,
    tiles::{Gem, Laser, LaserSource, Tile},
};

use super::pydirection::PyDirection;

#[pyclass(name = "Gem")]
pub struct PyGem {
    gem: Gem,
}

impl PyGem {
    pub fn new(gem: Gem) -> Self {
        PyGem { gem }
    }
}

#[pymethods]
impl PyGem {
    #[getter]
    fn is_collected(&self) -> bool {
        self.gem.is_collected()
    }

    #[getter]
    fn agent(&self) -> Option<AgentId> {
        self.gem.agent()
    }
}

#[pyclass(name = "Laser", unsendable)]
pub struct PyLaser {
    laser: Laser,
}

impl PyLaser {
    pub fn new(laser: Laser) -> Self {
        PyLaser { laser }
    }
}

#[pymethods]
impl PyLaser {
    #[getter]
    fn is_on(&self) -> bool {
        self.laser.is_on()
    }

    #[getter]
    fn is_off(&self) -> bool {
        self.laser.is_off()
    }

    #[getter]
    fn direction(&self) -> PyDirection {
        PyDirection::new(self.laser.direction())
    }

    #[getter]
    fn agent_id(&self) -> AgentId {
        self.laser.agent_id()
    }

    #[getter]
    fn agent(&self) -> Option<AgentId> {
        self.laser.agent()
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
