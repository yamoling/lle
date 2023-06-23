use std::{cell::RefCell, rc::Rc};

use super::Tile;

#[derive(Clone, Copy, Debug)]
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

impl From<&str> for Direction {
    fn from(value: &str) -> Self {
        match value {
            "N" => Direction::North,
            "E" => Direction::East,
            "S" => Direction::South,
            "W" => Direction::West,
            _ => panic!("Invalid direction: {value}"),
        }
    }
}

#[derive(Clone)]
pub struct Laser {
    is_on: Rc<RefCell<bool>>,
    direction: Direction,
    agent_num: u32,
    wrapped: Box<Tile>,
    next_tiles_status: Vec<Rc<RefCell<bool>>>,
}

impl Laser {
    pub fn new(
        direction: Direction,
        agent_num: u32,
        wrapped: Box<Tile>,
        is_on: Rc<RefCell<bool>>,
        next_tiles_status: Vec<Rc<RefCell<bool>>>,
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
        *self.is_on.borrow_mut() = true;
        self.wrapped.reset();
    }

    pub fn is_on(&self) -> bool {
        *self.is_on.borrow()
    }

    pub fn turn_off(&self) {
        *self.is_on.borrow_mut() = false;
        for status in &self.next_tiles_status {
            *status.borrow_mut() = false;
        }
    }

    pub fn turn_on(&self) {
        *self.is_on.borrow_mut() = true;
        for status in &self.next_tiles_status {
            *status.borrow_mut() = false;
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
