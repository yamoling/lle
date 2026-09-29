use crate::{
    WorldEvent,
    agent::{Agent, AgentId},
};

#[derive(Default, Debug)]
pub struct Gem {
    agent: Option<AgentId>,
    collected: bool,
}

impl Gem {
    pub fn is_collected(&self) -> bool {
        self.collected
    }

    pub fn collect(&mut self) {
        self.collected = true;
    }

    pub fn reset(&mut self) {
        self.collected = false;
        self.agent = None;
    }

    pub fn enter(&mut self, agent: &mut Agent) -> Option<WorldEvent> {
        self.agent = Some(agent.id());
        if !self.collected {
            self.collected = true;
            return Some(WorldEvent::GemCollected {
                agent_id: agent.id(),
            });
        }
        None
    }

    pub fn leave(&mut self) -> AgentId {
        self.agent.take().unwrap()
    }

    pub fn agent(&self) -> Option<AgentId> {
        self.agent
    }
}
