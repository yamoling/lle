use std::cell::Cell;

use crate::RewardEvent;

use super::{
    RewardCollector, REWARD_AGENT_DIED, REWARD_AGENT_JUST_ARRIVED, REWARD_END_GAME,
    REWARD_GEM_COLLECTED,
};

#[derive(Debug)]
pub struct TeamReward {
    step_reward: Cell<f32>,
    n_dead: Cell<u32>,
    episode_gems_collected: Cell<u32>,
    episode_agents_arrived: Cell<u32>,
    n_agents: u32,
}

impl TeamReward {
    pub fn new(n_agents: u32) -> Self {
        Self {
            step_reward: Cell::new(0f32),
            n_dead: Cell::new(0),
            episode_gems_collected: Cell::new(0),
            episode_agents_arrived: Cell::new(0),
            n_agents,
        }
    }
}

impl Clone for TeamReward {
    fn clone(&self) -> Self {
        Self {
            step_reward: Cell::new(self.step_reward.get()),
            n_dead: Cell::new(self.n_dead.get()),
            episode_gems_collected: Cell::new(self.episode_gems_collected.get()),
            episode_agents_arrived: Cell::new(self.episode_agents_arrived.get()),
            n_agents: self.n_agents,
        }
    }
}

impl RewardCollector for TeamReward {
    fn update(&self, event: RewardEvent) {
        // If the agent has died,
        if let RewardEvent::AgentDied { .. } = &event {
            // increase the number of dead agents,
            self.n_dead.set(self.n_dead.get() + 1);
            // cap the current reward to 0,
            let r = self.step_reward.get().min(0f32);
            // and add the reward for the agent dying.
            self.step_reward.set(r + REWARD_AGENT_DIED);
            return;
        }

        // Otherwise, if an agent has already died, we don't give any positive reward.
        if self.n_dead.get() > 0 {
            return;
        }

        // Last (general) case: all agents are alive
        let event_reward = match event {
            RewardEvent::AgentExit { .. } => {
                self.episode_agents_arrived
                    .set(self.episode_agents_arrived.get() + 1);
                if self.episode_agents_arrived.get() == self.n_agents {
                    REWARD_AGENT_JUST_ARRIVED + REWARD_END_GAME
                } else {
                    REWARD_AGENT_JUST_ARRIVED
                }
            }
            RewardEvent::GemCollected { .. } => {
                self.episode_gems_collected
                    .set(self.episode_gems_collected.get() + 1);
                REWARD_GEM_COLLECTED
            }
            RewardEvent::AgentDied { .. } => unreachable!(),
        };
        self.step_reward.set(self.step_reward.get() + event_reward);
    }

    fn reset(&self) {
        self.n_dead.set(0);
        self.step_reward.set(0f32);
        self.episode_agents_arrived.set(0);
        self.episode_gems_collected.set(0);
    }

    fn consume(&self) -> f32 {
        let n_deads = self.n_dead.get();
        let reward = self.step_reward.get();

        self.n_dead.set(0);
        self.step_reward.set(0f32);

        if n_deads > 0 {
            return -(n_deads as f32);
        }
        reward
    }
}
