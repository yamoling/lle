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
pub use core::{tiles, tiles::Tile, ParseError, RuntimeWorldError, World, WorldEvent, WorldState};
pub use rendering::Renderer;

// Include the version number of the crate from the build script
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
