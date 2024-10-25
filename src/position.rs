use std::ops::Add;

use serde::{Deserialize, Serialize};

use crate::{tiles::Direction, Action, RuntimeWorldError};

#[derive(Debug, Clone, Copy, Eq, Hash, Deserialize, Serialize)]
pub struct Position {
    pub i: usize,
    pub j: usize,
}

impl Position {
    pub fn as_xy(&self) -> (usize, usize) {
        (self.j, self.i)
    }

    pub fn as_ij(&self) -> (usize, usize) {
        (self.i, self.j)
    }

    pub fn x(&self) -> usize {
        self.j
    }

    pub fn y(&self) -> usize {
        self.i
    }
}

impl Add<Direction> for Position {
    type Output = Position;

    fn add(self, rhs: Direction) -> Self::Output {
        let (dx, dy) = rhs.delta();
        Self::Output {
            i: (self.i as i32 + dx) as usize,
            j: (self.j as i32 + dy) as usize,
        }
    }
}

impl Add<&Action> for &Position {
    type Output = Result<Position, RuntimeWorldError>;

    fn add(self, rhs: &Action) -> Self::Output {
        let (di, dj) = rhs.delta();
        let i = self.i as i32 + di;
        let j = self.j as i32 + dj;

        if j < 0 || i < 0 {
            return Err(RuntimeWorldError::OutOfWorldPosition {
                position: Position {
                    j: j as usize,
                    i: i as usize,
                },
            });
        }
        Ok(Position {
            j: j as usize,
            i: i as usize,
        })
    }
}

impl Into<(usize, usize)> for &Position {
    fn into(self) -> (usize, usize) {
        (self.i, self.j)
    }
}

impl PartialEq<Position> for Position {
    fn eq(&self, other: &Position) -> bool {
        self.i == other.i && self.j == other.j
    }
}

impl PartialEq<(usize, usize)> for Position {
    fn eq(&self, other: &(usize, usize)) -> bool {
        self.i == other.0 && self.j == other.1
    }
}
