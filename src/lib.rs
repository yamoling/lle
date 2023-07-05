mod action;
mod agent;
mod bindings;
mod errors;
mod levels;
// mod planning;
pub mod rendering;
mod reward_collector;
mod tiles;
mod utils;
mod world;

/// Position with (i, j) coordinates
pub type Position = (usize, usize);

pub use action::Action;
pub use agent::AgentId;
pub use errors::{RuntimeWorldError, WorldError};
pub use rendering::Renderer;
pub use reward_collector::{
    REWARD_AGENT_DIED, REWARD_AGENT_JUST_ARRIVED, REWARD_END_GAME, REWARD_GEM_COLLECTED,
};
pub use tiles::{Exit, Floor, Gem, Laser, LaserSource, Start, Tile, Wall};
pub use world::World;
