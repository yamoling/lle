use std::{
    collections::HashSet,
    sync::{Arc, Mutex},
};

use itertools::enumerate;
use pyo3::prelude::*;
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pymethods};

use crate::{
    Position, Tile, World,
    agent::Colour,
    bindings::{PyPosition, tiles::PyDirection},
    tiles::{LaserId, LaserSource},
};

#[gen_stub_pyclass]
#[pyclass(name = "LaserSource", module = "lle.tiles", from_py_object)]
#[derive(Clone)]
pub struct PyLaserSource {
    /// The colour of the laser: every agent of this colour can block and cross it.
    colour: Colour,
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
            colour: source.colour(),
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
        let tile = world.at_mut(&self.pos.into()).unwrap();
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

    /// The colour of the laser: every agent of this colour can block and cross it.
    #[getter]
    pub fn colour(&self) -> Colour {
        self.colour
    }

    /// Deprecated alias for `colour`, kept for one release.
    #[getter]
    pub fn agent_id(&self) -> Colour {
        self.colour
    }

    /// Setter form of [`Self::set_colour`], so that `source.colour = c` works too.
    #[setter(colour)]
    pub fn colour_setter(&mut self, new_colour: usize) -> PyResult<()> {
        self.set_colour(new_colour)
    }

    /// Deprecated setter alias, so that `source.agent_id = c` still works.
    #[setter(agent_id)]
    pub fn agent_id_setter(&mut self, new_colour: usize) -> PyResult<()> {
        self.set_colour(new_colour)
    }

    /// Change the laser's colour. Every agent of the new colour can then block and cross it.
    ///
    /// A colour need not correspond to an agent, so there is no upper bound on it: what is
    /// checked is that the beam would not cross the start position of an agent of *another*
    /// colour, which would kill that agent on reset.
    pub fn set_colour(&mut self, new_colour: usize) -> PyResult<()> {
        let world = self.world.lock().unwrap();
        if let Some(Tile::LaserSource(laser_source)) = world.at(&self.pos.into()) {
            laser_source.set_colour(new_colour as Colour);
        } else {
            return Err(pyo3::exceptions::PyValueError::new_err(
                "Tile is not a LaserSource",
            ));
        }
        // We have to check that the laser does not cross a start position
        // of an agent of a different colour.
        let lasers_positions: HashSet<Position> = world
            .lasers()
            .into_iter()
            .filter(|(_, l)| l.laser_id() == self.laser_id)
            .map(|(pos, _)| pos)
            .collect();
        let agent_colours = world.agent_colours();
        for (start_agent_id, pos) in enumerate(world.possible_starts()) {
            if agent_colours.get(start_agent_id) != Some(&new_colour) {
                let starts_set = HashSet::from_iter(pos);
                let intersection: Vec<_> = lasers_positions.intersection(&starts_set).collect();
                if !intersection.is_empty() {
                    return Err(pyo3::exceptions::PyValueError::new_err(format!(
                        "Laser source cannot be changed to colour {new_colour} since it would cross the start position of agent {start_agent_id} at {intersection:?}",
                    )));
                }
            }
        }
        self.colour = new_colour as Colour;
        Ok(())
    }

    /// Deprecated alias for `set_colour`, kept for one release.
    pub fn set_agent_id(&mut self, new_agent_id: usize) -> PyResult<()> {
        self.set_colour(new_agent_id)
    }

    /// Equality is based on the agent ID, direction, laser ID, and position.
    /// Whether a laser source is enabled is not considered.
    pub fn __eq__(&self, py: Python, other: Py<PyAny>) -> bool {
        if let Ok(source) = other.extract::<PyLaserSource>(py) {
            return self.colour == source.colour
                && self.direction == source.direction
                && self.laser_id == source.laser_id
                && self.pos == source.pos;
        }
        false
    }

    /// Hash based on the `laser_id`.
    pub fn __hash__(&self) -> usize {
        self.laser_id
    }

    pub fn __str__(&self) -> String {
        format!(
            "LaserSource(laser_id={}, is_enabled={}, direction={}, colour={})",
            self.laser_id,
            self.is_enabled,
            self.direction.name(),
            self.colour
        )
    }

    pub fn __repr__(&self) -> String {
        self.__str__()
    }
}
