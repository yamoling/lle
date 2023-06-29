use std::any::Any;

use crate::{
    agent::{Agent, AgentId},
    rendering::TileVisitor,
};

use super::{tile::TileClone, Floor, Tile};

#[derive(Debug, Clone)]
pub struct Start {
    floor: Floor,
    agent_id: AgentId,
}

impl Start {
    pub fn new(agent_id: AgentId) -> Self {
        Self {
            floor: Floor::default(),
            agent_id,
        }
    }

    pub fn agent_id(&self) -> AgentId {
        self.agent_id
    }
}

impl Tile for Start {
    fn pre_enter(&self, agent: &Agent) {
        self.floor.pre_enter(agent);
    }

    fn reset(&mut self) {
        self.floor.reset();
    }

    fn enter(&mut self, agent: &mut Agent) {
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
        visitor.visit_start(self, x, y);
    }
}

impl TileClone for Start {
    fn clone_box(&self) -> Box<dyn Tile> {
        Box::new(self.clone())
    }
}

#[derive(Debug, Clone, Default)]
pub struct Exit {
    floor: Floor,
}

impl Tile for Exit {
    fn pre_enter(&self, agent: &Agent) {
        self.floor.pre_enter(agent);
    }

    fn reset(&mut self) {
        self.floor.reset();
    }

    fn enter(&mut self, agent: &mut Agent) {
        self.floor.enter(agent);
        agent.arrive();
    }

    fn leave(&mut self) -> AgentId {
        panic!("Cannot leave an exit tile")
    }

    fn agent(&self) -> Option<AgentId> {
        self.floor.agent()
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn accept(&self, visitor: &mut dyn TileVisitor, x: u32, y: u32) {
        visitor.visit_exit(x, y);
    }
}

impl TileClone for Exit {
    fn clone_box(&self) -> Box<dyn Tile> {
        Box::new(self.clone())
    }
}
