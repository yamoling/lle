use crate::{
    agent::{Agent, AgentId},
    rendering::{TileVisitor, VisitorData},
    RuntimeWorldError, WorldEvent,
};
use core::panic;

pub trait Tile {
    fn pre_enter(&mut self, _agent: &Agent) -> Result<(), RuntimeWorldError> {
        if !self.is_waklable() {
            return Err(RuntimeWorldError::TileNotWalkable);
        }
        Ok(())
    }
    fn reset(&mut self);
    fn enter(&mut self, agent: &mut Agent) -> Option<WorldEvent>;
    fn leave(&mut self) -> AgentId;
    fn agent(&self) -> Option<AgentId>;
    fn is_waklable(&self) -> bool {
        true
    }
    fn is_occupied(&self) -> bool {
        self.agent().is_some()
    }
    /// Visitor pattern to render the tile
    fn accept(&self, visitor: &dyn TileVisitor, data: &mut VisitorData);
    fn to_string(&self) -> String;
}

#[derive(Default, Debug)]
pub struct Floor {
    agent: Option<AgentId>,
}

impl Tile for Floor {
    fn reset(&mut self) {
        self.agent = None;
    }

    fn enter(&mut self, agent: &mut Agent) -> Option<WorldEvent> {
        self.agent = Some(agent.id());
        None
    }

    fn leave(&mut self) -> AgentId {
        self.agent.take().expect("Can not call leave() because there is no agent on this tile.\nMaybe you forgot to perform a world.reset()?")
    }

    fn agent(&self) -> Option<AgentId> {
        self.agent
    }

    fn accept(&self, _visitor: &dyn TileVisitor, _data: &mut VisitorData) {
        // Nothing to do
    }

    fn to_string(&self) -> String {
        ".".to_string()
    }
}

#[derive(Default, Clone, Debug)]
pub struct Wall {}

impl TryFrom<&str> for Wall {
    type Error = String;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        if value == "@" {
            Ok(Wall {})
        } else {
            Err("Invalid wall character".into())
        }
    }
}

impl Tile for Wall {
    fn reset(&mut self) {}

    fn enter(&mut self, _agent: &mut Agent) -> Option<WorldEvent> {
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

    fn is_waklable(&self) -> bool {
        false
    }

    fn accept(&self, _visitor: &dyn TileVisitor, _data: &mut VisitorData) {
        // Nothing to do here as it is statically rendered
    }

    fn to_string(&self) -> String {
        "@".to_string()
    }
}

#[derive(Default)]
pub struct Void {
    agent: Option<AgentId>,
}

impl Tile for Void {
    fn agent(&self) -> Option<AgentId> {
        self.agent
    }

    fn enter(&mut self, agent: &mut Agent) -> Option<WorldEvent> {
        self.agent = Some(agent.id());
        if agent.is_alive() {
            agent.die();
            return Some(WorldEvent::AgentDied {
                agent_id: agent.id(),
            });
        }
        None
    }

    fn leave(&mut self) -> AgentId {
        panic!("Cannot leave a void: the game should be over !")
    }
    fn is_waklable(&self) -> bool {
        true
    }

    fn reset(&mut self) {
        self.agent = None;
    }

    fn accept(&self, _visitor: &dyn TileVisitor, _data: &mut VisitorData) {
        // Nothing to do
    }

    fn to_string(&self) -> String {
        "V".to_string()
    }
}

#[cfg(test)]
#[path = "../../unit_tests/test_tile.rs"]
mod tests;
