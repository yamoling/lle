use crate::{
    WorldEvent,
    agent::{self, Agent, AgentId},
};
/// A pressure-plate-like tile. Standing on it does nothing by itself; taking
/// `Action::Trigger` while standing on it pulses every `Lift` sharing the
/// same `group_id` (see `Lift` and `World::step`). Buttons hold no
/// persistent on/off state - the "pulse" model means every trigger is a
/// fresh one-shot event.
#[derive(Debug, Clone)]
pub struct Button {
    authorized_agent_id: Option<AgentId>,
    agent: Option<AgentId>,
    group_id: usize,
}

impl Button {
    pub fn new(group_id: usize) -> Self {
        Self {
            authorized_agent_id: None,
            agent: None,
            group_id,
        }
    }

    pub fn group_id(&self) -> usize {
        self.group_id
    }

    pub fn enter(&mut self, agent: &mut Agent) -> Option<WorldEvent> {
        self.agent = Some(agent.id());
        None
    }

    pub fn leave(&mut self) -> AgentId {
        self.agent.take().expect("No agent to leave")
    }

    pub fn reset(&mut self) {
        self.agent = None;
    }

    pub fn agent(&self) -> Option<AgentId> {
        self.agent
    }

    pub fn authorized_agent_id(&self) -> Option<AgentId> {
        self.authorized_agent_id
    }

    /// Dispatched by `Tile::actuate`. Returns the lift group this button
    /// belongs to, so `World` knows which `Lift` tiles to notify this tick.
    pub fn actuate(&mut self) -> Option<usize> {
        let agent_id = self.agent()?;

        if self
            .authorized_agent_id
            .is_some_and(|auth_id| auth_id != agent_id)
        {
            return None;
        }

        Some(self.group_id)
    }
}
