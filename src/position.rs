use std::ops::Add;

use serde::{Deserialize, Serialize};

use crate::{Action, RuntimeWorldError, tiles::Direction};

#[derive(Debug, Clone, Copy, Eq, Hash, Deserialize, Serialize)]
pub struct Position {
    pub i: usize,
    pub j: usize,
    pub k: usize,
}

impl Position {
    pub fn new2d(i: usize, j: usize) -> Self {
        Self { i, j, k: 0 }
    }

    pub fn as_xyz(&self) -> (usize, usize, usize) {
        (self.j, self.k, self.i)
    }

    pub fn as_ijk(&self) -> (usize, usize, usize) {
        (self.i, self.j, self.k)
    }

    pub fn x(&self) -> usize {
        self.j
    }

    pub fn y(&self) -> usize {
        self.i
    }

    pub fn z(&self) -> usize {
        self.k
    }
}

impl Add<Direction> for Position {
    type Output = Position;

    fn add(self, rhs: Direction) -> Self::Output {
        let (dx, dy) = rhs.delta();
        Self::Output {
            i: (self.i as i32 + dx) as usize,
            j: (self.j as i32 + dy) as usize,
            k: self.k,
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
                    k: self.k,
                },
            });
        }
        Ok(Position {
            j: j as usize,
            i: i as usize,
            k: self.k,
        })
    }
}

impl Into<(usize, usize, usize)> for &Position {
    fn into(self) -> (usize, usize, usize) {
        (self.i, self.j, self.k)
    }
}

impl Into<Position> for (usize, usize) {
    // simply for convenience
    fn into(self) -> Position {
        Position {
            i: self.0,
            j: self.1,
            k: 0,
        }
    }
}

impl PartialEq<Position> for Position {
    fn eq(&self, other: &Position) -> bool {
        self.i == other.i && self.j == other.j && self.k == other.k
    }
}

impl PartialEq<(usize, usize, usize)> for Position {
    fn eq(&self, other: &(usize, usize, usize)) -> bool {
        self.i == other.0 && self.j == other.1 && self.k == other.2
    }
}

impl PartialEq<(usize, usize)> for Position {
    fn eq(&self, other: &(usize, usize)) -> bool {
        self.i == other.0 && self.j == other.1 && self.k == 0
    }
}
