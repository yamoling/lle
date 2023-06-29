use crate::{
    agent::{Agent, AgentId},
    rendering::TileVisitor,
};
use std::{any::Any, fmt::Debug};

pub trait Tile: Debug + TileClone {
    fn pre_enter(&self, agent: &Agent);
    fn reset(&mut self);
    fn enter(&mut self, agent: &mut Agent);
    fn leave(&mut self) -> AgentId;
    fn agent(&self) -> Option<AgentId>;
    fn is_waklable(&self) -> bool {
        true
    }
    fn is_occupied(&self) -> bool {
        self.agent().is_some()
    }
    /// Visitor pattern to render the tile
    fn accept(&self, visitor: &mut dyn TileVisitor, x: u32, y: u32);

    // Required for testing purposes
    fn as_any(&self) -> &dyn Any;
}

pub trait TileClone {
    fn clone_box(&self) -> Box<dyn Tile>;
}

impl Clone for Box<dyn Tile> {
    fn clone(&self) -> Self {
        self.clone_box()
    }
}

#[derive(Debug, Clone, Default)]
pub struct Floor {
    agent: Option<AgentId>,
}

impl Tile for Floor {
    fn pre_enter(&self, _agent: &Agent) {}

    fn reset(&mut self) {
        self.agent = None;
    }

    fn enter(&mut self, agent: &mut Agent) {
        self.agent = Some(agent.id());
    }

    fn leave(&mut self) -> AgentId {
        self.agent.take().unwrap()
    }

    fn agent(&self) -> Option<AgentId> {
        self.agent
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn accept(&self, _visitor: &mut dyn TileVisitor, _x: u32, _y: u32) {
        // Nothing to do
    }
}

impl TileClone for Floor {
    fn clone_box(&self) -> Box<dyn Tile> {
        Box::new(self.clone())
    }
}

#[derive(Debug, Clone)]
pub struct Wall {}

impl Wall {
    pub fn new() -> Self {
        Self {}
    }
}

impl Tile for Wall {
    fn pre_enter(&self, _agent: &Agent) {
        panic!("Cannot pre-enter a wall")
    }

    fn reset(&mut self) {}

    fn enter(&mut self, _agent: &mut Agent) {
        panic!("Cannot enter a wall")
    }

    fn leave(&mut self) -> AgentId {
        panic!("Cannot leave a wall")
    }

    fn is_occupied(&self) -> bool {
        true
    }

    fn agent(&self) -> Option<AgentId> {
        None
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn accept(&self, visitor: &mut dyn TileVisitor, x: u32, y: u32) {
        visitor.visit_wall(x, y);
    }
}

impl TileClone for Wall {
    fn clone_box(&self) -> Box<dyn Tile> {
        Box::new(self.clone())
    }
}
