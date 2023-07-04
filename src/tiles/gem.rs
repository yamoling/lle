use std::cell::Cell;

use crate::{
    agent::{Agent, AgentId},
    rendering::{TileVisitor, VisitorData},
};

use super::{tile::TileClone, Floor, Tile};

#[derive(Debug, Clone, Default)]
pub struct Gem {
    floor: Floor,
    collected: Cell<bool>,
}

impl Gem {
    pub fn is_collected(&self) -> bool {
        self.collected.get()
    }

    pub fn collect(&self) {
        self.collected.set(true);
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
            agent.collect_gem();
            self.collect();
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

impl TileClone for Gem {
    fn clone_box(&self) -> Box<dyn Tile> {
        Box::new(self.clone())
    }
}
