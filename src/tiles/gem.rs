use std::{cell::Cell, rc::Rc};

use crate::{
    agent::{Agent, AgentId},
    rendering::{TileVisitor, VisitorData},
    reward::RewardCollector,
    RewardEvent,
};

use super::{Floor, Tile};

pub struct Gem {
    floor: Floor,
    collected: Cell<bool>,
    reward_collector: Rc<dyn RewardCollector>,
}

impl Gem {
    pub fn new(reward_collector: Rc<dyn RewardCollector>) -> Self {
        Self {
            floor: Floor::default(),
            collected: Cell::new(false),
            reward_collector,
        }
    }

    pub fn collect(&self) {
        self.collected.set(true);
    }

    pub fn is_collected(&self) -> bool {
        self.collected.get()
    }
}

impl Tile for Gem {
    fn pre_enter(&self, agent: &Agent) {
        self.floor.pre_enter(agent);
    }

    fn reset(&self) {
        self.collected.set(false);
        self.floor.reset();
    }

    fn enter(&self, agent: &mut Agent) {
        if !self.collected.get() {
            self.collected.set(true);
            self.reward_collector.update(RewardEvent::GemCollected {
                agent_id: agent.id(),
            });
        }
        self.floor.enter(agent);
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
