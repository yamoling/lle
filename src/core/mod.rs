mod errors;
mod event;
mod levels;
mod parsing;
mod world;
mod world_state;

pub use errors::RuntimeWorldError;
pub use event::WorldEvent;
pub use parsing::ParseError;
pub use world::World;
pub use world_state::WorldState;

pub const REWARD_GEM_COLLECTED: f32 = 1f32;
pub const REWARD_AGENT_DIED: f32 = -1f32;
pub const REWARD_AGENT_EXIT: f32 = 1f32;
pub const REWARD_END_GAME: f32 = 1f32;
