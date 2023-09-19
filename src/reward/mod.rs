mod independent_reward;
mod reward_event;
mod team_reward;

pub const REWARD_GEM_COLLECTED: i32 = 1;
pub const REWARD_AGENT_DIED: i32 = -1;
pub const REWARD_AGENT_JUST_ARRIVED: i32 = 1;
pub const REWARD_END_GAME: i32 = 1;

pub use reward_event::RewardEvent;
pub use team_reward::TeamReward;
