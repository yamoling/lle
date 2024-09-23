use std::sync::{Arc, Mutex};

use pyo3::prelude::*;
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pymethods};

use crate::{
    agent::AgentId,
    bindings::pydirection::PyDirection,
    tiles::{Laser, LaserId},
    Position, Tile, World,
};

use super::inner;

/// A laser tile of the world.
#[gen_stub_pyclass]
#[pyclass(name = "Laser", module = "lle.tiles")]
pub struct PyLaser {
    /// The ID of the laser (unique per laser source)
    #[pyo3(get)]
    laser_id: LaserId,
    /// The id of the agent that can block the laser.
    #[pyo3(get)]
    agent_id: AgentId,
    /// The direction of the laser beam.
    #[pyo3(get)]
    direction: PyDirection,
    /// Whether the laser is turned on.
    #[pyo3(get)]
    is_on: bool,
    /// Whether the laser is enabled.
    #[pyo3(get)]
    is_enabled: bool,
    pos: Position,
    world: Arc<Mutex<World>>,
}

unsafe impl Send for PyLaser {}
unsafe impl Sync for PyLaser {}

impl PyLaser {
    pub fn new(laser: &Laser, pos: Position, world: Arc<Mutex<World>>) -> Self {
        Self {
            laser_id: laser.laser_id(),
            agent_id: laser.agent_id(),
            direction: PyDirection::from(laser.direction()),
            is_on: laser.is_on(),
            is_enabled: laser.is_enabled(),
            pos,
            world,
        }
    }
}

#[gen_stub_pymethods]
#[pymethods]
impl PyLaser {
    /// Whether the laser is turned off.
    #[getter]
    pub fn is_off(&self) -> bool {
        !self.is_on
    }

    /// Whether the laser is disabled.
    #[getter]
    pub fn is_disabled(&self) -> bool {
        !self.is_enabled
    }

    /// The id of the agent currently standing on the tile, if any.
    #[getter]
    pub fn agent(&self) -> Option<AgentId> {
        let world = &mut self.world.lock().unwrap();
        let tile = inner(world, self.pos).unwrap();
        match tile {
            Tile::Laser(laser) => laser.agent(),
            _ => None,
        }
    }

    pub fn __str__(&self) -> String {
        let agent = match self.agent() {
            Some(agent) => agent.to_string(),
            None => String::from("None"),
        };

        format!(
            "Laser(laser_id={}, is_on={}, direction={}, agent_id={}, agent={agent})",
            self.laser_id,
            self.is_on,
            self.direction.name(),
            self.agent_id
        )
    }

    pub fn __repr__(&self) -> String {
        self.__str__()
    }
}
