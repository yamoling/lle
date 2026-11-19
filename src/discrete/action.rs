use std::{
    fmt::{Display, Formatter},
    ops::Add,
    slice::Iter,
};

use crate::{Position, RuntimeWorldError};

#[derive(Debug, Clone, PartialEq, PartialOrd, Eq, Ord, Copy)]
pub enum Action {
    North = 0,
    South = 1,
    East = 2,
    West = 3,
    Stay = 4,
    Trigger = 5,
}

impl Action {
    pub fn delta(&self) -> (i32, i32) {
        match self {
            Action::North => (-1, 0),
            Action::South => (1, 0),
            Action::East => (0, 1),
            Action::West => (0, -1),
            Action::Stay => (0, 0),
            Action::Trigger => (0, 0),
        }
    }

    /// The action that moves an agent from `from` to the adjacent cell `to` on the same layer.
    ///
    /// `between(p, p)` is [`Action::Stay`], and the four neighbours of `from` give
    /// [`Action::North`] (`i - 1`), [`Action::South`], [`Action::East`] and [`Action::West`].
    ///
    /// Adjacency is checked on the `usize` coordinates, so distant cells never alias to an
    /// action and no coordinate overflows.
    ///
    /// # Errors
    /// - [`RuntimeWorldError::InvalidActionDelta`] with `(di, dj) = to - from` when `to` is on the
    ///   same layer as `from` but is neither `from` nor one of its four neighbours;
    /// - [`RuntimeWorldError::PositionsNotAdjacent`] when `to` is on another layer than `from`,
    ///   or when `di` or `dj` does not fit in an `i32`.
    ///
    /// @ai-generated
    pub fn between(from: Position, to: Position) -> Result<Action, RuntimeWorldError> {
        let not_adjacent = RuntimeWorldError::PositionsNotAdjacent {
            pos_0: from,
            pos1: to,
        };
        if from.k != to.k {
            return Err(not_adjacent);
        }
        if from.j == to.j {
            if from.i == to.i {
                return Ok(Action::Stay);
            }
            if from.i.checked_sub(1) == Some(to.i) {
                return Ok(Action::North);
            }
            if from.i.checked_add(1) == Some(to.i) {
                return Ok(Action::South);
            }
        } else if from.i == to.i {
            if from.j.checked_add(1) == Some(to.j) {
                return Ok(Action::East);
            }
            if from.j.checked_sub(1) == Some(to.j) {
                return Ok(Action::West);
            }
        }
        match (signed_delta(from.i, to.i), signed_delta(from.j, to.j)) {
            (Some(di), Some(dj)) => Err(RuntimeWorldError::InvalidActionDelta { di, dj }),
            _ => Err(not_adjacent),
        }
    }

    pub fn opposite(&self) -> Action {
        match self {
            Action::North => Action::South,
            Action::South => Action::North,
            Action::East => Action::West,
            Action::West => Action::East,
            Action::Stay => Action::Stay,
            Action::Trigger => Action::Stay,
        }
    }
}

/// `to - from` as an `i32`, or `None` when it does not fit.
///
/// @ai-generated
fn signed_delta(from: usize, to: usize) -> Option<i32> {
    if to >= from {
        i32::try_from(to - from).ok()
    } else {
        i32::try_from(from - to).ok().map(|d| -d)
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
            Action::Trigger,
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
            5 => Action::Trigger,
            _ => panic!("Invalid value for action: {}", value),
        }
    }
}

impl TryFrom<(i32, i32)> for Action {
    type Error = RuntimeWorldError;

    fn try_from(value: (i32, i32)) -> Result<Self, Self::Error> {
        match value {
            (-1, 0) => Ok(Action::North),
            (1, 0) => Ok(Action::South),
            (0, 1) => Ok(Action::East),
            (0, -1) => Ok(Action::West),
            (0, 0) => Ok(Action::Stay),
            (di, dj) => Err(RuntimeWorldError::InvalidActionDelta { di, dj }),
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
            "Trigger" | "T" => Action::Trigger,
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

#[cfg(test)]
#[path = "../unit_tests/test_action_between.rs"]
mod test_action_between;
