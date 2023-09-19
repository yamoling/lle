use std::{fmt::Display, rc::Rc};

use crate::reward_collector::{RewardEvent, SharedRewardCollector};

pub type AgentId = usize;

#[derive(Debug, Clone)]
pub struct Agent {
    id: AgentId,
    dead: bool,
    arrived: bool,
    reward_collector: Rc<SharedRewardCollector>,
}

impl Agent {
    pub fn new(id: u32, collector: Rc<SharedRewardCollector>) -> Self {
        Self {
            id: id as usize,
            dead: false,
            arrived: false,
            reward_collector: collector,
        }
    }

    pub fn reset(&mut self) {
        self.dead = false;
        self.arrived = false;
        self.reward_collector.reset();
    }

    pub fn die(&mut self) {
        self.dead = true;
        self.reward_collector
            .notify(RewardEvent::AgentDied { agent_id: self.id });
    }

    pub fn arrive(&mut self) {
        self.arrived = true;
        self.reward_collector
            .notify(RewardEvent::JustArrived { agent_id: self.id });
    }

    pub fn collect_gem(&self) {
        self.reward_collector
            .notify(RewardEvent::GemCollected { agent_id: self.id });
    }

    pub fn has_arrived(&self) -> bool {
        self.arrived
    }

    pub fn is_dead(&self) -> bool {
        self.dead
    }

    pub fn is_alive(&self) -> bool {
        !self.dead
    }

    pub fn id(&self) -> AgentId {
        self.id
    }
}

impl Display for Agent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("Agent {}", self.id))
    }
}
