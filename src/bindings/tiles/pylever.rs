use std::sync::{Arc, Mutex};

use pyo3::prelude::*;
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pymethods};

use crate::{
    Position, World,
    agent::{AgentId, Colour},
    bindings::PyPosition,
    tiles::{Lever, Tile},
};

use super::inner;

/// A toggle tile: taking `Action.TRIGGER` while standing on it switches it on
/// or off. While on, it pulses every `Lift` sharing the same `group_id` at
/// every step, even once its toggler has left.
#[gen_stub_pyclass]
#[pyclass(name = "Lever", module = "lle.tiles")]
pub struct PyLever {
    /// The group shared with the `Lift`(s) this lever can pulse.
    #[pyo3(get)]
    group_id: usize,
    /// If set, only agents of this colour can toggle the lever.
    #[pyo3(get)]
    authorized_colour: Option<Colour>,
    /// The (i, j, k) position of the lever.
    #[pyo3(get)]
    pos: PyPosition,
    world: Arc<Mutex<World>>,
}

unsafe impl Send for PyLever {}
unsafe impl Sync for PyLever {}

impl PyLever {
    pub fn new(lever: &Lever, pos: Position, world: Arc<Mutex<World>>) -> Self {
        Self {
            group_id: lever.group_id(),
            authorized_colour: lever.authorized_colour(),
            pos: PyPosition::from(pos),
            world,
        }
    }
}

#[gen_stub_pymethods]
#[pymethods]
impl PyLever {
    /// The id of the agent currently standing on the tile, if any.
    #[getter]
    pub fn agent(&self) -> Option<AgentId> {
        let world = &mut self.world.lock().unwrap();
        // `Tile::agent` also sees through the laser beams covering the tile.
        inner(world, self.pos.into()).unwrap().agent()
    }

    /// Whether the lever is currently on, i.e. pulsing its group at every step.
    #[getter]
    pub fn is_on(&self) -> bool {
        let world = &mut self.world.lock().unwrap();
        match inner(world, self.pos.into()).unwrap().innermost() {
            Tile::Lever(lever) => lever.is_on(),
            _ => unreachable!("A lever position should hold a lever"),
        }
    }

    pub fn __str__(&self) -> String {
        format!(
            "Lever(pos={:?}, group_id={}, authorized_colour={:?}, agent={:?}, is_on={})",
            self.pos,
            self.group_id,
            self.authorized_colour,
            self.agent(),
            self.is_on()
        )
    }

    pub fn __repr__(&self) -> String {
        self.__str__()
    }
}
