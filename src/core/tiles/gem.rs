use std::cell::Cell;

use crate::{
    agent::{Agent, AgentId},
    rendering::{TileVisitor, VisitorData},
    WorldEvent,
};

use super::{Floor, Tile};

#[derive(Default)]
pub struct Gem {
    floor: Floor,
    collected: Cell<bool>,
}

impl Gem {
    pub fn collect(&self) {
        self.collected.set(true);
    }

    pub fn is_collected(&self) -> bool {
        self.collected.get()
    }
}

impl Tile for Gem {
    fn pre_enter(&self, agent: &Agent) -> Result<(), String> {
        self.floor.pre_enter(agent)
    }

    fn reset(&self) {
        self.collected.set(false);
        self.floor.reset();
    }

    fn enter(&self, agent: &mut Agent) -> Option<WorldEvent> {
        self.floor.enter(agent);
        if !self.collected.get() {
            self.collected.set(true);
            return Some(WorldEvent::GemCollected {
                agent_id: agent.id(),
            });
        }
        None
    }

    fn leave(&self) -> AgentId {
        self.floor.leave()
    }

    fn agent(&self) -> Option<AgentId> {
        self.floor.agent()
    }

    fn accept(&self, visitor: &dyn TileVisitor, data: &mut VisitorData) {
        visitor.visit_gem(self, data);
    }
}
