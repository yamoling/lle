use std::sync::atomic::{AtomicBool, Ordering};

use crate::{
    agent::{Agent, AgentId},
    rendering::{TileVisitor, VisitorData},
    RuntimeWorldError, WorldEvent,
};

use super::{Floor, Tile};

#[derive(Default, Debug)]
pub struct Gem {
    floor: Floor,
    collected: AtomicBool,
}

impl Gem {
    pub fn collect(&self) {
        self.collected.store(true, Ordering::Relaxed);
    }

    pub fn is_collected(&self) -> bool {
        self.collected.load(Ordering::Relaxed)
    }
}

impl Tile for Gem {
    fn pre_enter(&mut self, agent: &Agent) -> Result<(), RuntimeWorldError> {
        self.floor.pre_enter(agent)
    }

    fn reset(&mut self) {
        self.collected.store(false, Ordering::Relaxed);
        self.floor.reset();
    }

    fn enter(&mut self, agent: &mut Agent) -> Option<WorldEvent> {
        self.floor.enter(agent);
        if !self.collected.load(Ordering::Relaxed) {
            self.collected.store(true, Ordering::Relaxed);
            return Some(WorldEvent::GemCollected {
                agent_id: agent.id(),
            });
        }
        None
    }

    fn leave(&mut self) -> AgentId {
        self.floor.leave()
    }

    fn agent(&self) -> Option<AgentId> {
        self.floor.agent()
    }

    fn accept(&self, visitor: &dyn TileVisitor, data: &mut VisitorData) {
        visitor.visit_gem(self, data);
    }
}
