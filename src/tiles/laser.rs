use std::{any::Any, cell::Cell, rc::Rc};

use crate::{
    agent::{Agent, AgentId},
    rendering::TileVisitor,
};

use super::{tile::TileClone, Tile};

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

impl TryFrom<&str> for Direction {
    type Error = &'static str;
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "N" => Ok(Direction::North),
            "E" => Ok(Direction::East),
            "S" => Ok(Direction::South),
            "W" => Ok(Direction::West),
            _ => Err("Invalid direction"),
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

#[derive(Debug, Clone)]
pub struct Laser {
    direction: Direction,
    agent_id: AgentId,
    beam: LaserBeam,
    wrapped: Box<dyn Tile>,
}

impl Laser {
    pub fn new(
        agent_id: AgentId,
        direction: Direction,
        wrapped: Box<dyn Tile>,
        beam: LaserBeam,
    ) -> Self {
        Self {
            agent_id,
            direction,
            wrapped,
            beam,
        }
    }

    pub fn agent_id(&self) -> AgentId {
        self.agent_id
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
}

impl Tile for Laser {
    fn pre_enter(&self, agent: &Agent) {
        self.wrapped.pre_enter(agent);
        if agent.id() == self.agent_id {
            self.beam.turn_off();
        }
    }

    fn reset(&mut self) {
        self.beam.turn_on();
        self.wrapped.reset();
    }

    fn enter(&mut self, agent: &mut Agent) {
        self.wrapped.enter(agent);
        // Note: turning off the beam happens in `pre_enter`
        if self.beam.is_on() && agent.id() != self.agent_id {
            agent.die();
        }
    }

    fn leave(&mut self) -> AgentId {
        self.beam.turn_off();
        self.wrapped.leave()
    }

    fn agent(&self) -> Option<AgentId> {
        self.wrapped.agent()
    }

    fn is_waklable(&self) -> bool {
        self.wrapped.is_waklable()
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn accept(&self, visitor: &mut dyn TileVisitor, x: u32, y: u32) {
        visitor.visit_laser(self, x, y);
    }
}

impl TileClone for Laser {
    fn clone_box(&self) -> Box<dyn Tile> {
        Box::new(self.clone())
    }
}
