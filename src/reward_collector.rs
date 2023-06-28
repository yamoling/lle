use std::cell::Cell;

pub struct RewardCollector {
    step_reward: Cell<i32>,
    n_dead: Cell<u32>,
    episode_gems_collected: Cell<u32>,
    episode_agents_arrived: Cell<u32>,
    n_agents: u32,
}

pub enum RewardEvent {
    JustArrived,
    GemCollected,
    AgentDied,
}

impl RewardCollector {
    pub fn new(n_agents: u32) -> Self {
        Self {
            step_reward: Cell::new(0),
            n_dead: Cell::new(0),
            episode_gems_collected: Cell::new(0),
            episode_agents_arrived: Cell::new(0),
            n_agents,
        }
    }

    pub fn notify(&self, event: RewardEvent) {
        let reward = self.step_reward.get();
        let event_reward = match event {
            RewardEvent::AgentDied => -1,
            RewardEvent::GemCollected => {
                self.episode_gems_collected
                    .set(self.episode_gems_collected.get() + 1);
                1
            }
            RewardEvent::JustArrived => {
                self.episode_agents_arrived
                    .set(self.episode_agents_arrived.get() + 1);
                let reward = 1;
                if self.episode_agents_arrived() == self.n_agents {
                    reward + 1
                } else {
                    reward
                }
            }
        };
        self.step_reward.set(reward + event_reward);
    }

    /// Reset the reward collector and return the reward for the step.
    pub fn consume_step_reward(&self) -> i32 {
        let n_deads = self.n_dead.get();
        let reward = self.step_reward.get();

        self.n_dead.set(0);
        self.step_reward.set(0);

        if n_deads > 0 {
            return -(n_deads as i32);
        }
        reward
    }

    pub fn episode_gems_collected(&self) -> u32 {
        self.episode_gems_collected.get()
    }

    pub fn episode_agents_arrived(&self) -> u32 {
        self.episode_agents_arrived.get()
    }

    pub fn reset(&self) {
        self.n_dead.set(0);
        self.step_reward.set(0);
        self.episode_agents_arrived.set(0);
        self.episode_gems_collected.set(0);
    }
}
