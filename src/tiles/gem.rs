use std::any::Any;

use crate::{
    agent::{Agent, AgentId},
    rendering::TileVisitor,
};

use super::{tile::TileClone, Floor, Tile};

#[derive(Debug, Clone, Default)]
pub struct Gem {
    floor: Floor,
    collected: bool,
}

impl Gem {
    pub fn is_collected(&self) -> bool {
        self.collected
    }
}

impl Tile for Gem {
    fn pre_enter(&self, agent: &Agent) {
        self.floor.pre_enter(agent);
    }

    fn reset(&mut self) {
        self.collected = false;
        self.floor.reset();
    }

    fn enter(&mut self, agent: &mut Agent) {
        if !self.collected {
            agent.collect_gem();
            self.collected = true;
        }
        self.floor.enter(agent);
    }

    fn leave(&mut self) -> AgentId {
        self.floor.leave()
    }

    fn agent(&self) -> Option<AgentId> {
        self.floor.agent()
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn accept(&self, visitor: &mut dyn TileVisitor, x: u32, y: u32) {
        visitor.visit_gem(self, x, y);
    }
}

impl TileClone for Gem {
    fn clone_box(&self) -> Box<dyn Tile> {
        Box::new(self.clone())
    }
}
