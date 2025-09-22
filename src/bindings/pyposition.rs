use crate::Position;

pub type PyPosition = (usize, usize, usize);

impl From<Position> for PyPosition {
    fn from(pos: Position) -> Self {
        pos.as_ijk()
    }
}

impl Into<Position> for PyPosition {
    fn into(self) -> Position {
        Position {
            i: self.0,
            j: self.1,
            k: self.2,
        }
    }
}

impl Into<Position> for (usize, usize) { // simply for convenience
    fn into(self) -> Position {
        Position {
            i: self.0,
            j: self.1,
            k: 0,
        }
    }
}