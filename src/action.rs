use std::{
    fmt::{Display, Formatter},
    ops::Add,
    slice::Iter,
};

use crate::{Position, RuntimeWorldError};

#[derive(Debug, Clone, PartialEq, PartialOrd, Eq, Ord)]
pub enum Action {
    North = 0,
    South = 1,
    East = 2,
    West = 3,
    Stay = 4,
}

impl Action {
    pub fn delta(&self) -> (i32, i32) {
        match self {
            Action::North => (-1, 0),
            Action::South => (1, 0),
            Action::East => (0, 1),
            Action::West => (0, -1),
            Action::Stay => (0, 0),
        }
    }

    pub fn opposite(&self) -> Action {
        match self {
            Action::North => Action::South,
            Action::South => Action::North,
            Action::East => Action::West,
            Action::West => Action::East,
            Action::Stay => Action::Stay,
        }
    }
}

impl Action {
    pub fn iter() -> Iter<'static, Action> {
        [
            Action::North,
            Action::South,
            Action::East,
            Action::West,
            Action::Stay,
        ]
        .iter()
    }
}

impl From<u32> for Action {
    fn from(value: u32) -> Self {
        match value {
            0 => Action::North,
            1 => Action::South,
            2 => Action::East,
            3 => Action::West,
            4 => Action::Stay,
            _ => panic!("Invalid value for action: {}", value),
        }
    }
}

impl From<&str> for Action {
    fn from(value: &str) -> Self {
        match value {
            "North" | "N" => Action::North,
            "South" | "S" => Action::South,
            "East" | "E" => Action::East,
            "West" | "W" => Action::West,
            "Stay" => Action::Stay,
            _ => panic!("Invalid value for action: {}", value),
        }
    }
}

impl Display for Action {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl Add<Position> for Action {
    type Output = Result<Position, RuntimeWorldError>;

    fn add(self, rhs: Position) -> Self::Output {
        &rhs + &self
    }
}

impl Add<&Position> for &Action {
    type Output = Result<Position, RuntimeWorldError>;

    fn add(self, rhs: &Position) -> Self::Output {
        rhs + self
    }
}
