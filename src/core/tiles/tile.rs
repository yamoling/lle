use crate::{
    agent::{Agent, AgentId},
    rendering::{TileVisitor, VisitorData},
    WorldEvent,
};
use core::panic;
use std::cell::Cell;

pub trait Tile {
    fn pre_enter(&self, _agent: &Agent) -> Result<(), String> {
        if !self.is_waklable() {
            return Err("Cannot walk on this tile".to_string());
        }
        Ok(())
    }
    fn reset(&self);
    fn enter(&self, agent: &mut Agent) -> Option<WorldEvent>;
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

#[derive(Default)]
pub struct Floor {
    agent: Cell<Option<AgentId>>,
}

impl Tile for Floor {
    fn reset(&self) {
        self.agent.set(None);
    }

    fn enter(&self, agent: &mut Agent) -> Option<WorldEvent> {
        self.agent.set(Some(agent.id()));
        None
    }

    fn leave(&self) -> AgentId {
        self.agent.take().expect("Can not call leave() because there is no agent on this tile.\nMaybe you forgot to perform a world.reset()?")
    }

    fn agent(&self) -> Option<AgentId> {
        self.agent.get()
    }

    fn accept(&self, _visitor: &dyn TileVisitor, _data: &mut VisitorData) {
        // Nothing to do
    }
}

#[derive(Default, Clone)]
pub struct Wall {}

impl Tile for Wall {
    fn pre_enter(&self, _agent: &Agent) -> Result<(), String> {
        Err("Cannot pre-enter a wall".into())
    }

    fn reset(&self) {}

    fn enter(&self, _agent: &mut Agent) -> Option<WorldEvent> {
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

#[derive(Default)]
pub struct Void {
    agent: Cell<Option<AgentId>>,
}

impl Tile for Void {
    fn agent(&self) -> Option<AgentId> {
        self.agent.get()
    }

    fn enter(&self, agent: &mut Agent) -> Option<WorldEvent> {
        self.agent.set(Some(agent.id()));
        if agent.is_alive() {
            agent.die();
            return Some(WorldEvent::AgentDied {
                agent_id: agent.id(),
            });
        }
        None
    }

    fn leave(&self) -> AgentId {
        panic!("Cannot leave a void: the game should be over !")
    }

    fn is_occupied(&self) -> bool {
        self.agent.get().is_some()
    }

    fn is_waklable(&self) -> bool {
        true
    }

    fn reset(&self) {
        self.agent.set(None);
    }

    fn accept(&self, _visitor: &dyn TileVisitor, _data: &mut VisitorData) {
        // Nothing to do
    }
}

#[cfg(test)]
#[path = "../../unit_tests/test_tile.rs"]
mod tests;
