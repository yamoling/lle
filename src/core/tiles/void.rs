use crate::{agent::Agent, AgentId, WorldEvent};

#[derive(Default)]
pub struct Void {
    agent: Option<AgentId>,
}

impl Void {
    pub fn agent(&self) -> Option<AgentId> {
        self.agent
    }

    pub fn enter(&mut self, agent: &mut Agent) -> Option<WorldEvent> {
        self.agent = Some(agent.id());
        if agent.is_alive() {
            agent.die();
            return Some(WorldEvent::AgentDied {
                agent_id: agent.id(),
            });
        }
        None
    }

    pub fn leave(&mut self) -> AgentId {
        self.agent.take().unwrap()
    }

    pub fn is_waklable(&self) -> bool {
        true
    }

    pub fn reset(&mut self) {
        self.agent = None;
    }
}
