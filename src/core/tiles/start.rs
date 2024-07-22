use crate::{
    agent::{Agent, AgentId},
    WorldEvent,
};

pub struct Start {
    agent: Option<AgentId>,
    start_agent_id: AgentId,
}

impl Start {
    pub fn new(start_agent_id: AgentId) -> Self {
        Self {
            agent: None,
            start_agent_id,
        }
    }

    pub fn reset(&mut self) {
        self.agent = None;
    }

    pub fn enter(&mut self, agent: &mut Agent) -> Option<WorldEvent> {
        self.agent = Some(agent.id());
        None
    }

    pub fn leave(&mut self) -> AgentId {
        self.agent.take().unwrap()
    }

    pub fn agent(&self) -> Option<AgentId> {
        self.agent
    }

    pub fn start_agent_id(&self) -> AgentId {
        self.start_agent_id
    }
}
