use pyo3::PyResult;

mod pydirection;
mod pygem;
mod pylaser;
mod pylaser_source;

pub use pydirection::PyDirection;
pub use pygem::PyGem;
pub use pylaser::PyLaser;
pub use pylaser_source::PyLaserSource;

use crate::{Position, Tile, World};

fn inner(world: &mut World, pos: Position) -> PyResult<&mut Tile> {
    match world.at_mut(&pos) {
        Some(tile) => Ok(tile),
        None => Err(pyo3::exceptions::PyValueError::new_err(format!(
            "Tile not found at {:?}",
            pos
        ))),
    }
}
