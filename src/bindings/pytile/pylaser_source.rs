use std::sync::{Arc, Mutex};

use pyo3::prelude::*;
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pymethods};

use crate::{
    agent::AgentId,
    bindings::{pydirection::PyDirection, pyposition::PyPosition},
    tiles::{LaserId, LaserSource},
    Tile, World,
};

#[gen_stub_pyclass]
#[pyclass(name = "LaserSource", module = "lle.tiles")]
pub struct PyLaserSource {
    /// The id (colour) of the agent that can block the laser.
    #[pyo3(get)]
    agent_id: AgentId,
    /// The direction of the laser beam.
    /// The direction can currently not be changed after creation of the `World`.
    #[pyo3(get)]
    direction: PyDirection,
    /// Whether the laser source is enabled.
    #[pyo3(get)]
    is_enabled: bool,
    /// The unique id of the laser.
    #[pyo3(get)]
    laser_id: LaserId,
    /// The (i, j) position of the laser tile.
    #[pyo3(get)]
    pos: PyPosition,
    world: Arc<Mutex<World>>,
}

unsafe impl Send for PyLaserSource {}
unsafe impl Sync for PyLaserSource {}

impl PyLaserSource {
    pub fn new(world: Arc<Mutex<World>>, pos: (usize, usize), source: &LaserSource) -> Self {
        Self {
            agent_id: source.agent_id(),
            direction: PyDirection::from(source.direction()),
            is_enabled: source.is_enabled(),
            laser_id: source.laser_id(),
            pos: pos.into(),
            world,
        }
    }

    fn set_status(&mut self, enabled: bool) {
        if self.is_enabled == enabled {
            return;
        }

        let world = &mut self.world.lock().unwrap();
        let tile = world.at_mut(&self.pos.clone().into()).unwrap();
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

#[gen_stub_pymethods]
#[pymethods]
impl PyLaserSource {
    /// Whether the laser source is disabled.
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

    /// Disable the laser source and its corresponding laser tiles.
    pub fn disable(&mut self) {
        self.set_status(false)
    }

    /// Enable the laser source and its corresponding laser tiles.
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
        if let Some(Tile::LaserSource(laser_source)) = world.at(&self.pos.clone().into()) {
            laser_source.set_agent_id(agent_id as AgentId);
            self.agent_id = agent_id as AgentId;
        } else {
            return Err(pyo3::exceptions::PyValueError::new_err(
                "Tile is not a LaserSource",
            ));
        }
        Ok(())
    }

    /// Change the colour of the laser to the one of the given agent ID.
    /// Alias to `source.agent_id = new_agent_id`.
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
