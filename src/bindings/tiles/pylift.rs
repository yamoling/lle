use std::sync::{Arc, Mutex};

use pyo3::prelude::*;
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pymethods};

use crate::{Position, Tile, World, agent::AgentId, bindings::PyPosition, tiles::Lift};

use super::inner;

/// A one-shot elevator tile: pressing a `Button` sharing the same `group_id`
/// relocates whichever agent stands on the lift by one step in its `direction`.
#[gen_stub_pyclass]
#[pyclass(name = "Lift", module = "lle.tiles")]
pub struct PyLift {
    /// The direction in which the lift moves its occupant: "U" (up a layer)
    /// or "D" (down a layer). A lift only ever moves an agent between
    /// layers, never sideways.
    #[pyo3(get)]
    direction: String,
    /// The group shared with the `Button`(s) that can pulse this lift.
    #[pyo3(get)]
    group_id: usize,
    /// If set, only this agent is relocated when the lift is pulsed.
    #[pyo3(get)]
    authorized_agent_id: Option<AgentId>,
    /// The (i, j, k) position of the lift.
    #[pyo3(get)]
    pos: PyPosition,
    world: Arc<Mutex<World>>,
}

unsafe impl Send for PyLift {}
unsafe impl Sync for PyLift {}

impl PyLift {
    pub fn new(lift: &Lift, pos: Position, world: Arc<Mutex<World>>) -> Self {
        Self {
            direction: lift.direction().to_file_string(),
            group_id: lift.group_id(),
            authorized_agent_id: lift.authorized_agent_id(),
            pos: PyPosition::from(pos),
            world,
        }
    }
}

#[gen_stub_pymethods]
#[pymethods]
impl PyLift {
    /// The id of the agent currently standing on the tile, if any.
    #[getter]
    pub fn agent(&self) -> Option<AgentId> {
        let world = &mut self.world.lock().unwrap();
        let tile = inner(world, self.pos.into()).unwrap();
        match tile {
            Tile::Lift(lift) => lift.agent(),
            _ => None,
        }
    }

    pub fn __str__(&self) -> String {
        format!(
            "Lift(pos={:?}, direction={}, group_id={}, authorized_agent_id={:?}, agent={:?})",
            self.pos,
            self.direction,
            self.group_id,
            self.authorized_agent_id,
            self.agent()
        )
    }

    pub fn __repr__(&self) -> String {
        self.__str__()
    }
}
