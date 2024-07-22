use std::{
    fmt::{self, Debug, Formatter},
    sync::{Arc, Mutex},
};

use pyo3::prelude::*;

use crate::{agent::AgentId, tiles::Gem, Position, Tile, World};

use super::inner;

#[pyclass(name = "Gem")]
pub struct PyGem {
    #[pyo3(get)]
    is_collected: bool,
    #[pyo3(get)]
    pos: Position,
    world: Arc<Mutex<World>>,
}

// PyGem is thread-safe because it wraps a thread-safe object
// and the other fields are immutable.
unsafe impl Send for PyGem {}
unsafe impl Sync for PyGem {}

impl PyGem {
    pub fn new(gem: &Gem, pos: Position, world: Arc<Mutex<World>>) -> Self {
        Self {
            is_collected: gem.is_collected(),
            pos,
            world,
        }
    }
}

#[pymethods]
impl PyGem {
    pub fn __str__(&self) -> String {
        format!("{self:?}")
    }

    pub fn __repr__(&self) -> String {
        self.__str__()
    }

    pub fn collect(&mut self) -> PyResult<()> {
        let world = &mut self.world.lock().unwrap();
        let tile = inner(world, self.pos)?;
        match tile {
            Tile::Gem(gem) => gem.collect(),
            _ => {
                return Err(pyo3::exceptions::PyValueError::new_err(format!(
                    "Tile at {:?} is not a gem",
                    self.pos
                )))
            }
        };
        self.is_collected = true;
        Ok(())
    }

    #[getter]
    pub fn agent(&self) -> Option<AgentId> {
        let world = self.world.lock().unwrap();
        let tile = world.at(self.pos)?;
        match tile {
            Tile::Gem(gem) => gem.agent(),
            _ => None,
        }
    }
}

impl Debug for PyGem {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Gem(pos={:?}, is_collected={}, agent={:?})",
            self.pos,
            self.is_collected,
            self.agent()
        )
    }
}
