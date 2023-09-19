use crate::{
    agent::{Agent, AgentId},
    rendering::{TileVisitor, VisitorData},
};

use super::{Direction, Tile, Wall};

#[derive(Clone)]
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

    fn reset(&self) {
        self.wall.reset();
    }

    fn enter(&self, agent: &mut Agent) {
        self.wall.enter(agent);
    }

    fn leave(&self) -> AgentId {
        self.wall.leave()
    }

    fn agent(&self) -> Option<AgentId> {
        self.wall.agent()
    }

    fn is_waklable(&self) -> bool {
        false
    }

    fn accept(&self, _visitor: &dyn TileVisitor, _data: &mut VisitorData) {
        // Nothing to do here as it is statically rendered
    }
}
