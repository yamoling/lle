use std::sync::{Arc, Mutex};

use pyo3::prelude::*;
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pymethods};

use crate::{Position, Tile, World, agent::AgentId, bindings::PyPosition, tiles::Button};

use super::inner;

/// A pressure-plate tile: taking `Action.TRIGGER` while standing on it
/// pulses every `Lift` sharing the same `group_id`.
#[gen_stub_pyclass]
#[pyclass(name = "Button", module = "lle.tiles")]
pub struct PyButton {
    /// The group shared with the `Lift`(s) this button can pulse.
    #[pyo3(get)]
    group_id: usize,
    /// If set, only this agent can actuate the button.
    #[pyo3(get)]
    authorized_agent_id: Option<AgentId>,
    /// The (i, j, k) position of the button.
    #[pyo3(get)]
    pos: PyPosition,
    world: Arc<Mutex<World>>,
}

unsafe impl Send for PyButton {}
unsafe impl Sync for PyButton {}

impl PyButton {
    pub fn new(button: &Button, pos: Position, world: Arc<Mutex<World>>) -> Self {
        Self {
            group_id: button.group_id(),
            authorized_agent_id: button.authorized_agent_id(),
            pos: PyPosition::from(pos),
            world,
        }
    }
}

#[gen_stub_pymethods]
#[pymethods]
impl PyButton {
    /// The id of the agent currently standing on the tile, if any.
    #[getter]
    pub fn agent(&self) -> Option<AgentId> {
        let world = &mut self.world.lock().unwrap();
        let tile = inner(world, self.pos.into()).unwrap();
        match tile {
            Tile::Button(button) => button.agent(),
            _ => None,
        }
    }

    pub fn __str__(&self) -> String {
        format!(
            "Button(pos={:?}, group_id={}, authorized_agent_id={:?}, agent={:?})",
            self.pos,
            self.group_id,
            self.authorized_agent_id,
            self.agent()
        )
    }

    pub fn __repr__(&self) -> String {
        self.__str__()
    }
}
