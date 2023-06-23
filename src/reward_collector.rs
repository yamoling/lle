use std::cell::RefCell;

#[derive(Default)]
pub struct RewardCollector {
    step_reward: RefCell<i32>,
    n_dead: RefCell<u32>,
}

pub enum RewardEvent {
    JustArrived,
    GemCollected,
    GameFinished,
    AgentDied,
}

impl RewardCollector {
    pub fn notify(&self, event: RewardEvent) {
        let mut reward = self.step_reward.borrow_mut();
        match event {
            RewardEvent::AgentDied => {
                *self.n_dead.borrow_mut() += 1;
            }
            RewardEvent::GemCollected => *reward += 1,
            RewardEvent::GameFinished => *reward += 1,
            RewardEvent::JustArrived => *reward += 1,
        }
    }

    /// Reset the reward collector and return the reward for the step.
    pub fn consume(&self) -> i32 {
        let n_deads = *self.n_dead.borrow();
        let reward = *self.step_reward.borrow();

        *self.n_dead.borrow_mut() = 0;
        *self.step_reward.borrow_mut() = 0;

        if n_deads > 0 {
            return -(n_deads as i32);
        }
        reward
    }
}
