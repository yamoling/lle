use serde::{Deserialize, Serialize};

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
        pos.as_ijk()
    }
}

impl PartialEq<(usize, usize, usize)> for Position {
    fn eq(&self, other: &(usize, usize, usize)) -> bool {
        self.i == other.0 && self.j == other.1 && self.k == other.2
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
