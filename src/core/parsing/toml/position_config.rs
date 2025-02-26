use serde::{Deserialize, Serialize};

use crate::{ParseError, Position};

#[derive(Deserialize, Serialize, Debug)]
#[serde(untagged)]
pub enum PositionsConfig {
    IJ {
        i: usize,
        j: usize,
    },
    Row {
        row: usize,
    },
    Column {
        col: usize,
    },
    Rect {
        #[serde(default)]
        i_min: usize,
        #[serde(default)]
        i_max: Option<usize>,
        #[serde(default)]
        j_min: usize,
        #[serde(default)]
        j_max: Option<usize>,
    },
}

impl PositionsConfig {
    pub fn to_positions(&self, width: usize, height: usize) -> Result<Vec<Position>, ParseError> {
        match self {
            Self::IJ { i, j } => {
                if *i >= height || *j >= width {
                    return Err(ParseError::PositionOutOfBounds { i: *i, j: *j });
                }
                Ok(vec![Position { i: *i, j: *j }])
            }
            Self::Rect {
                i_min,
                i_max,
                j_min,
                j_max,
            } => {
                if *i_min >= height || *j_min >= width {
                    return Err(ParseError::PositionOutOfBounds {
                        i: *i_min,
                        j: *j_min,
                    });
                }
                let mut positions = vec![];
                for i in *i_min..=i_max.unwrap_or(height - 1) {
                    for j in *j_min..=j_max.unwrap_or(width - 1) {
                        if i >= height || j >= width {
                            return Err(ParseError::PositionOutOfBounds { i, j });
                        }
                        positions.push(Position { i, j });
                    }
                }
                Ok(positions)
            }
            Self::Row { row } => {
                if *row >= height {
                    return Err(ParseError::PositionOutOfBounds { i: *row, j: 0 });
                }
                Ok((0..width).map(|j| Position { i: *row, j }).collect())
            }
            Self::Column { col } => {
                if *col >= width {
                    return Err(ParseError::PositionOutOfBounds { i: 0, j: *col });
                }
                Ok((0..height).map(|i| Position { i, j: *col }).collect())
            }
        }
    }
}

impl From<&Position> for PositionsConfig {
    fn from(pos: &Position) -> Self {
        Self::IJ { i: pos.i, j: pos.j }
    }
}
