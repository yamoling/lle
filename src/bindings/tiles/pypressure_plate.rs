use std::sync::{Arc, Mutex};

use pyo3::prelude::*;
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pymethods};

use crate::{
    Position, World,
    agent::{AgentId, Colour},
    bindings::PyPosition,
    tiles::PressurePlate,
};

use super::inner;

/// A passive tile: it pulses every `Lift` sharing the same `group_id` at every
/// step in which an authorized agent stands on it. No `Action.TRIGGER` is needed.
#[gen_stub_pyclass]
#[pyclass(name = "PressurePlate", module = "lle.tiles")]
pub struct PyPressurePlate {
    /// The group shared with the `Lift`(s) this plate can pulse.
    #[pyo3(get)]
    group_id: usize,
    /// If set, only agents of this colour activate the plate.
    #[pyo3(get)]
    authorized_colour: Option<Colour>,
    /// The (i, j, k) position of the plate.
    #[pyo3(get)]
    pos: PyPosition,
    world: Arc<Mutex<World>>,
}

unsafe impl Send for PyPressurePlate {}
unsafe impl Sync for PyPressurePlate {}

impl PyPressurePlate {
    pub fn new(plate: &PressurePlate, pos: Position, world: Arc<Mutex<World>>) -> Self {
        Self {
            group_id: plate.group_id(),
            authorized_colour: plate.authorized_colour(),
            pos: PyPosition::from(pos),
            world,
        }
    }
}

#[gen_stub_pymethods]
#[pymethods]
impl PyPressurePlate {
    /// The id of the agent currently standing on the tile, if any.
    #[getter]
    pub fn agent(&self) -> Option<AgentId> {
        let world = &mut self.world.lock().unwrap();
        // `Tile::agent` also sees through the laser beams covering the tile.
        inner(world, self.pos.into()).unwrap().agent()
    }

    pub fn __str__(&self) -> String {
        format!(
            "PressurePlate(pos={:?}, group_id={}, authorized_colour={:?}, agent={:?})",
            self.pos,
            self.group_id,
            self.authorized_colour,
            self.agent()
        )
    }

    pub fn __repr__(&self) -> String {
        self.__str__()
    }
}
