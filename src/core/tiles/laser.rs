use std::{cell::Cell, fmt::Display, rc::Rc};

use crate::{
    agent::{Agent, AgentId},
    rendering::{TileVisitor, VisitorData},
    WorldEvent,
};

use super::Tile;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Direction {
    North,
    East,
    South,
    West,
}

impl Direction {
    pub fn delta(&self) -> (i32, i32) {
        match self {
            Direction::North => (-1, 0),
            Direction::East => (0, 1),
            Direction::South => (1, 0),
            Direction::West => (0, -1),
        }
    }
}

impl Display for Direction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

impl TryFrom<&str> for Direction {
    type Error = String;
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "N" => Ok(Direction::North),
            "E" => Ok(Direction::East),
            "S" => Ok(Direction::South),
            "W" => Ok(Direction::West),
            other => Err(format!(
                "Invalid direction: {other}. Expected one of {{N, E, S, W}}."
            )),
        }
    }
}

#[derive(Debug, Clone)]
pub struct LaserBeam {
    beam: Vec<Rc<Cell<bool>>>,
}

impl LaserBeam {
    pub fn new(beam: Vec<Rc<Cell<bool>>>) -> Self {
        Self { beam }
    }

    pub fn is_on(&self) -> bool {
        self.beam[0].get()
    }

    pub fn is_off(&self) -> bool {
        !self.is_on()
    }

    pub fn turn_on(&self) {
        for cell in &self.beam {
            cell.set(true);
        }
    }

    pub fn turn_off(&self) {
        for cell in &self.beam {
            cell.set(false);
        }
    }
}

pub struct Laser {
    direction: Direction,
    agent_id: Cell<AgentId>,
    beam: LaserBeam,
    wrapped: Rc<dyn Tile>,
}

impl Laser {
    pub fn new(
        agent_id: AgentId,
        direction: Direction,
        wrapped: Rc<dyn Tile>,
        beam: LaserBeam,
    ) -> Self {
        Self {
            agent_id: Cell::new(agent_id),
            direction,
            wrapped,
            beam,
        }
    }

    pub fn agent_id(&self) -> AgentId {
        self.agent_id.get()
    }

    pub fn set_agent_id(&self, agent_id: AgentId) {
        self.agent_id.set(agent_id);
    }

    pub fn wrapped(&self) -> &dyn Tile {
        self.wrapped.as_ref()
    }

    pub fn is_on(&self) -> bool {
        self.beam.is_on()
    }

    pub fn is_off(&self) -> bool {
        self.beam.is_off()
    }

    pub fn direction(&self) -> Direction {
        self.direction
    }

    pub fn turn_on(&self) {
        self.beam.turn_on();
    }

    pub fn turn_off(&self) {
        self.beam.turn_off();
    }
}

impl Tile for Laser {
    fn reset(&self) {
        self.turn_on();
        self.wrapped.reset();
    }

    fn pre_enter(&self, agent: &Agent) -> Result<(), String> {
        let res = self.wrapped.pre_enter(agent);
        if agent.is_alive() && agent.id() == self.agent_id.get() {
            self.turn_off();
        }
        res
    }

    fn enter(&self, agent: &mut Agent) -> Option<WorldEvent> {
        // Note: turning off the beam happens in `pre_enter`
        if self.is_on() && agent.id() != self.agent_id.get() {
            if agent.is_alive() {
                agent.die();
                self.beam.turn_on();
                return Some(WorldEvent::AgentDied {
                    agent_id: agent.id(),
                });
            }
            return None;
        }
        self.wrapped.enter(agent)
    }

    fn leave(&self) -> AgentId {
        self.turn_on();
        self.wrapped.leave()
    }

    fn agent(&self) -> Option<AgentId> {
        self.wrapped.agent()
    }

    fn is_waklable(&self) -> bool {
        self.wrapped.is_waklable()
    }

    fn accept(&self, visitor: &dyn TileVisitor, data: &mut VisitorData) {
        visitor.visit_laser(self, data);
    }
}
