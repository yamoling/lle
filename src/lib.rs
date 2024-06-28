mod action;
mod agent;
mod bindings;
mod core;
pub mod rendering;

mod utils;

/// Position with (i, j) coordinates
pub type Position = (usize, usize);

pub use action::Action;
pub use agent::AgentId;
use core::tiles::Direction;
pub use core::{tiles, tiles::Tile, ParseError, RuntimeWorldError, World, WorldEvent, WorldState};
pub use rendering::Renderer;
use std::ops::Add;

// Include the version number of the crate from the build script
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

impl Add<Direction> for Position {
    type Output = Position;

    fn add(self, rhs: Direction) -> Self::Output {
        let (dx, dy) = rhs.delta();
        ((self.0 as i32 + dx) as usize, (self.1 as i32 + dy) as usize)
    }
}
