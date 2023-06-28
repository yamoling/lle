use std::{cell::Cell, rc::Rc};

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

#[derive(Clone, Debug)]
pub struct Laser {
    is_on: Rc<Cell<bool>>,
    direction: Direction,
    agent_num: u32,
    wrapped: Box<Tile>,
    next_tiles_status: Vec<Rc<Cell<bool>>>,
}

impl Laser {
    pub fn new(
        direction: Direction,
        agent_num: u32,
        wrapped: Box<Tile>,
        is_on: Rc<Cell<bool>>,
        next_tiles_status: Vec<Rc<Cell<bool>>>,
    ) -> Self {
        Self {
            is_on,
            direction,
            agent_num,
            wrapped,
            next_tiles_status,
        }
    }

    pub fn reset(&mut self) {
        self.is_on.set(true);
        self.wrapped.reset();
    }

    pub fn is_on(&self) -> bool {
        self.is_on.get()
    }

    pub fn is_off(&self) -> bool {
        !self.is_on.get()
    }

    pub fn turn_off(&self) {
        self.is_on.set(false);
        for status in &self.next_tiles_status {
            status.set(false);
        }
    }

    pub fn turn_on(&self) {
        self.is_on.set(true);
        for status in &self.next_tiles_status {
            status.set(true);
        }
    }

    pub fn direction(&self) -> Direction {
        self.direction
    }

    pub fn agent_num(&self) -> u32 {
        self.agent_num
    }

    pub fn wrapped(&self) -> &Tile {
        &self.wrapped
    }
}
