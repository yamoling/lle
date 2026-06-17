use crate::Position;

pub type PyPosition = (usize, usize);

impl From<Position> for PyPosition {
    fn from(pos: Position) -> Self {
        pos.as_ij()
    }
}
