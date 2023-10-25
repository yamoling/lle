mod independent_reward;
mod reward_event;
mod team_reward;

pub const REWARD_GEM_COLLECTED: f32 = 1f32;
pub const REWARD_AGENT_DIED: f32 = -1f32;
pub const REWARD_AGENT_JUST_ARRIVED: f32 = 1f32;
pub const REWARD_END_GAME: f32 = 1f32;

pub use reward_event::RewardEvent;
pub use team_reward::TeamReward;

pub trait RewardModel {
    /// Update the reward of this step according to the given event.
    fn update(&self, event: RewardEvent);
    /// Consume and return the reward of this time step.
    /// The reward model is then ready for the next time step.
    fn consume(&self) -> f32;
    /// Completely reset the reward model.
    fn reset(&self);
}
