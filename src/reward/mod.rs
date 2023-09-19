mod independent_reward;
mod reward_event;
mod team_reward;

pub const REWARD_GEM_COLLECTED: f32 = 1f32;
pub const REWARD_AGENT_DIED: f32 = -1f32;
pub const REWARD_AGENT_JUST_ARRIVED: f32 = 1f32;
pub const REWARD_END_GAME: f32 = 1f32;

pub use reward_event::RewardEvent;
pub use team_reward::TeamReward;

pub trait RewardCollector {
    fn update(&self, event: RewardEvent);
    fn consume(&self) -> f32;
    fn reset(&self);
}
