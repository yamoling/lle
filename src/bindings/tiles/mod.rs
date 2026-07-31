use pyo3::PyResult;

mod pybutton;
mod pydirection;
mod pygem;
mod pylaser;
mod pylaser_source;
mod pylift;

pub use pybutton::PyButton;
pub use pydirection::PyDirection;
pub use pygem::PyGem;
pub use pylaser::PyLaser;
pub use pylaser_source::PyLaserSource;
pub use pylift::PyLift;

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
