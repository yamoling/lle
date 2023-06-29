use std::any::Any;

use crate::{
    agent::{Agent, AgentId},
    rendering::TileVisitor,
};

use super::{tile::TileClone, Direction, Tile, Wall};

#[derive(Debug, Clone)]
pub struct LaserSource {
    wall: Wall,
    direction: Direction,
    agent_id: AgentId,
}

impl LaserSource {
    pub fn new(direction: Direction, agent_id: AgentId) -> Self {
        Self {
            wall: Wall {},
            direction,
            agent_id,
        }
    }

    pub fn agent_id(&self) -> AgentId {
        self.agent_id
    }

    pub fn direction(&self) -> Direction {
        self.direction
    }
}

impl Tile for LaserSource {
    fn pre_enter(&self, agent: &Agent) {
        self.wall.pre_enter(agent);
    }

    fn reset(&mut self) {
        self.wall.reset();
    }

    fn enter(&mut self, agent: &mut Agent) {
        self.wall.enter(agent);
    }

    fn leave(&mut self) -> AgentId {
        self.wall.leave()
    }

    fn agent(&self) -> Option<AgentId> {
        self.wall.agent()
    }

    fn is_waklable(&self) -> bool {
        false
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn accept(&self, visitor: &mut dyn TileVisitor, x: u32, y: u32) {
        visitor.visit_laser_source(self, x, y);
    }
}

impl TileClone for LaserSource {
    fn clone_box(&self) -> Box<dyn Tile> {
        Box::new(self.clone())
    }
}
