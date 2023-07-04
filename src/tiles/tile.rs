use crate::{
    agent::{Agent, AgentId},
    rendering::{TileVisitor, VisitorData},
};
use std::{cell::Cell, fmt::Debug};

pub trait Tile: Debug + TileClone {
    fn pre_enter(&self, agent: &Agent);
    fn reset(&self);
    fn enter(&self, agent: &mut Agent);
    fn leave(&self) -> AgentId;
    fn agent(&self) -> Option<AgentId>;
    fn is_waklable(&self) -> bool {
        true
    }
    fn is_occupied(&self) -> bool {
        self.agent().is_some()
    }
    /// Visitor pattern to render the tile
    fn accept(&self, visitor: &dyn TileVisitor, data: &mut VisitorData);
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
    agent: Cell<Option<AgentId>>,
}

impl Tile for Floor {
    fn pre_enter(&self, _agent: &Agent) {}

    fn reset(&self) {
        self.agent.set(None);
    }

    fn enter(&self, agent: &mut Agent) {
        self.agent.set(Some(agent.id()));
    }

    fn leave(&self) -> AgentId {
        self.agent.take().unwrap()
    }

    fn agent(&self) -> Option<AgentId> {
        self.agent.get()
    }

    fn accept(&self, _visitor: &dyn TileVisitor, _data: &mut VisitorData) {
        // Nothing to do
    }
}

impl TileClone for Floor {
    fn clone_box(&self) -> Box<dyn Tile> {
        Box::new(self.clone())
    }
}

#[derive(Debug, Clone, Default)]
pub struct Wall {}

impl Tile for Wall {
    fn pre_enter(&self, _agent: &Agent) {
        panic!("Cannot pre-enter a wall")
    }

    fn reset(&self) {}

    fn enter(&self, _agent: &mut Agent) {
        panic!("Cannot enter a wall")
    }

    fn leave(&self) -> AgentId {
        panic!("Cannot leave a wall")
    }

    fn is_occupied(&self) -> bool {
        true
    }

    fn agent(&self) -> Option<AgentId> {
        None
    }

    fn is_waklable(&self) -> bool {
        false
    }

    fn accept(&self, _visitor: &dyn TileVisitor, _data: &mut VisitorData) {
        // Nothing to do here as it is statically rendered
    }
}

impl TileClone for Wall {
    fn clone_box(&self) -> Box<dyn Tile> {
        Box::new(self.clone())
    }
}

#[cfg(test)]
#[path = "../unit_tests/test_tile.rs"]
mod tests;
