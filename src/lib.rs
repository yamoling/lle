mod action;
mod agent;
mod bindings;
mod core;
pub mod rendering;
mod tiles;
mod utils;

/// Position with (i, j) coordinates
pub type Position = (usize, usize);

pub use action::Action;
pub use agent::AgentId;
pub use core::{ParseError, RuntimeWorldError, World, WorldEvent, WorldState};
pub use rendering::Renderer;
pub use tiles::{Exit, Floor, Gem, Laser, LaserSource, Start, Tile, Wall};

// Include the version number of the crate from the build script
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
