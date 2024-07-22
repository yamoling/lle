use std::sync::{Arc, Mutex};

use pyo3::prelude::*;

use crate::{
    agent::AgentId,
    bindings::pydirection::PyDirection,
    tiles::{LaserId, LaserSource},
    Position, Tile, World,
};

#[pyclass(name = "LaserSource", module = "lle")]
pub struct PyLaserSource {
    #[pyo3(get)]
    agent_id: AgentId,
    #[pyo3(get)]
    direction: PyDirection,
    #[pyo3(get)]
    is_enabled: bool,
    #[pyo3(get)]
    laser_id: LaserId,
    #[pyo3(get)]
    pos: Position,
    world: Arc<Mutex<World>>,
}

unsafe impl Send for PyLaserSource {}
unsafe impl Sync for PyLaserSource {}

impl PyLaserSource {
    pub fn new(world: Arc<Mutex<World>>, pos: Position, source: &LaserSource) -> Self {
        Self {
            agent_id: source.agent_id(),
            direction: PyDirection::from(source.direction()),
            is_enabled: source.is_enabled(),
            laser_id: source.laser_id(),
            pos,
            world,
        }
    }

    fn set_status(&mut self, enabled: bool) {
        if self.is_enabled == enabled {
            return;
        }

        let world = &mut self.world.lock().unwrap();
        let tile = world.at_mut(self.pos).unwrap();
        // let tile = inner(world, self.pos).unwrap();
        match tile {
            Tile::LaserSource(laser_source) => {
                if enabled {
                    laser_source.enable();
                } else {
                    laser_source.disable();
                }
                self.is_enabled = enabled;
            }
            _ => panic!("Tile at {:?} is not a LaserSource", self.pos),
        }
    }
}

#[pymethods]
impl PyLaserSource {
    #[getter]
    pub fn is_disabled(&self) -> bool {
        !self.is_enabled
    }

    #[setter]
    pub fn set_is_enabled(&mut self, enabled: bool) {
        self.set_status(enabled)
    }

    #[setter]
    pub fn set_is_disabled(&mut self, disabled: bool) {
        self.set_status(!disabled)
    }

    pub fn disable(&mut self) {
        self.set_status(false)
    }

    pub fn enable(&mut self) {
        self.set_status(true)
    }

    #[setter]
    pub fn set_agent_id(&mut self, agent_id: i32) -> PyResult<()> {
        if agent_id < 0 {
            return Err(pyo3::exceptions::PyValueError::new_err(
                "Agent ID must be positive",
            ));
        }
        let world = self.world.lock().unwrap();
        let agent_id = agent_id as usize;
        if agent_id >= world.n_agents() {
            return Err(pyo3::exceptions::PyValueError::new_err(
                "Agent ID is greater than the number of agents",
            ));
        }
        if let Some(Tile::LaserSource(laser_source)) = world.at(self.pos) {
            laser_source.set_agent_id(agent_id as AgentId);
            self.agent_id = agent_id as AgentId;
        } else {
            return Err(pyo3::exceptions::PyValueError::new_err(
                "Tile is not a LaserSource",
            ));
        }
        Ok(())
    }

    fn set_colour(&mut self, colour: i32) -> PyResult<()> {
        self.set_agent_id(colour)
    }

    pub fn __str__(&self) -> String {
        format!(
            "LaserSource(laser_id={}, is_enabled={}, direction={}, agent_id={})",
            self.laser_id,
            self.is_enabled,
            self.direction.name(),
            self.agent_id
        )
    }

    pub fn __repr__(&self) -> String {
        self.__str__()
    }
}
