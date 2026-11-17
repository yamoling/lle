//! Moving a [`Position`] by a direction or an action. These operations live in the discrete engine
//! because they report out-of-world moves with a [`RuntimeWorldError`].
use std::ops::Add;

use crate::{
    discrete::{Action, RuntimeWorldError},
    shared::{CardinalDirection, Direction, Position, VerticalDirection},
};

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
