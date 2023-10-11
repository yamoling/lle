mod action;
mod agent;
mod bindings;
mod parsing;
//mod planning;
// mod observations;
pub mod rendering;
pub mod reward;
mod tiles;
mod utils;
mod world;

/// Position with (i, j) coordinates
pub type Position = (usize, usize);

pub use action::Action;
pub use agent::AgentId;
pub use parsing::{parse, ParseError};
pub use rendering::Renderer;
pub use reward::{
    RewardEvent, TeamReward, REWARD_AGENT_DIED, REWARD_AGENT_JUST_ARRIVED, REWARD_END_GAME,
    REWARD_GEM_COLLECTED,
};
pub use tiles::{Exit, Floor, Gem, Laser, LaserSource, Start, Tile, Wall};
pub use world::{RuntimeWorldError, World, WorldState};

// Include the version number of the crate from the build script
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
