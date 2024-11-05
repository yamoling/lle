use std::{
    fmt::{self, Debug, Formatter},
    sync::{Arc, Mutex},
};

use pyo3::prelude::*;
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pymethods};

use crate::{agent::AgentId, tiles::Gem, Tile, World};

use super::{super::pyposition::PyPosition, inner};

#[gen_stub_pyclass]
#[pyclass(name = "Gem", module = "lle.tiles")]
pub struct PyGem {
    /// Whether the gem has been collected.
    #[pyo3(get)]
    is_collected: bool,
    /// The (i, j) position of the gem.
    #[pyo3(get)]
    pos: PyPosition,
    world: Arc<Mutex<World>>,
}

// PyGem is thread-safe because it wraps a thread-safe object
// and the other fields are immutable.
unsafe impl Send for PyGem {}
unsafe impl Sync for PyGem {}

impl PyGem {
    pub fn new(gem: &Gem, pos: (usize, usize), world: Arc<Mutex<World>>) -> Self {
        Self {
            is_collected: gem.is_collected(),
            pos: pos.into(),
            world,
        }
    }
}

#[gen_stub_pymethods]
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
        let tile = inner(world, self.pos.clone().into())?;
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
        let tile = world.at(&self.pos.clone().into())?;
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
