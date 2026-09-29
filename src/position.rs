use std::ops::{Add, Sub};

use serde::{Deserialize, Serialize};

use crate::{
    Action, RuntimeWorldError,
    tiles::{CardinalDirection, Direction, VerticalDirection},
};

#[derive(Debug, Clone, Copy, Eq, Hash, PartialEq, Deserialize, Serialize)]
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

    pub fn as_ij(&self) -> (usize, usize) {
        (self.i, self.j)
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
    type Output = Result<Position, RuntimeWorldError>;

    fn add(self, rhs: Direction) -> Self::Output {
        let (dx, dy, dz) = rhs.delta();
        let i = self.i as i32 + dx;
        let j = self.j as i32 + dy;
        let k = self.k as i32 + dz;

        if j < 0 || i < 0 || k < 0 {
            return Err(RuntimeWorldError::OutOfWorldPosition {
                position: Position {
                    j: j as usize,
                    i: i as usize,
                    k: k as usize,
                },
            });
        }
        Ok(Position {
            i: i as usize,
            j: j as usize,
            k: k as usize,
        })
    }
}

impl Add<CardinalDirection> for Position {
    type Output = Result<Position, RuntimeWorldError>;

    fn add(self, rhs: CardinalDirection) -> Self::Output {
        self + Direction::from(rhs)
    }
}

impl Add<VerticalDirection> for Position {
    type Output = Result<Position, RuntimeWorldError>;

    fn add(self, rhs: VerticalDirection) -> Self::Output {
        self + Direction::from(rhs)
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

impl From<(usize, usize, usize)> for Position {
    fn from((i, j, k): (usize, usize, usize)) -> Self {
        Self { i, j, k }
    }
}

impl From<(usize, usize)> for Position {
    fn from((i, j): (usize, usize)) -> Self {
        Self { i, j, k: 0 }
    }
}

impl From<Position> for (usize, usize, usize) {
    fn from(pos: Position) -> Self {
        let (i, j, k) = pos.as_ijk();
        (i, j, k)
    }
}

impl PartialEq<(usize, usize, usize)> for Position {
    fn eq(&self, other: &(usize, usize, usize)) -> bool {
        self.i == other.0 && self.j == other.1 && self.k == other.2
    }
}

impl Sub<Position> for Position {
    type Output = Result<Action, RuntimeWorldError>;

    fn sub(self, rhs: Position) -> Self::Output {
        let di = self.i as i32 - rhs.i as i32;
        let dj = self.j as i32 - rhs.j as i32;
        Action::try_from((di, dj))
    }
}

impl From<&Position> for (usize, usize) {
    fn from(val: &Position) -> Self {
        (val.i, val.j)
    }
}

impl PartialEq<(usize, usize)> for Position {
    fn eq(&self, other: &(usize, usize)) -> bool {
        self.i == other.0 && self.j == other.1 && self.k == 0
    }
}
