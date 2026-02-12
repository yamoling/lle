use crate::Position;

pub type PyPosition = (usize, usize);

impl From<Position> for PyPosition {
    fn from(pos: Position) -> Self {
        pos.as_ij()
    }
}

impl Into<Position> for PyPosition {
    fn into(self) -> Position {
        Position {
            i: self.0,
            j: self.1,
        }
    }
}
