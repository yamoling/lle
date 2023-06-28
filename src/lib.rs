mod action;
mod agent;
mod bindings;
mod errors;
mod rendering;
mod reward_collector;
mod tiles;
mod utils;
mod world;

/// Position with (i, j) coordinates
pub type Position = (usize, usize);

pub use action::Action;
pub use errors::WorldError;
pub use rendering::Renderer;
pub use world::World;
