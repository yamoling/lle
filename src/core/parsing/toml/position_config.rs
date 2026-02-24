use itertools::Itertools;
use serde::{Deserialize, Serialize};

use crate::{ParseError, Position};

#[derive(Deserialize, Serialize, Debug)]
#[serde(untagged)]
pub enum PositionsConfig {
    IJK {
        i: usize,
        j: usize,
        k: usize,
    },
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
    Layer {
        lay: usize,
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
        #[serde(default)]
        k_min: usize,
        #[serde(default)]
        k_max: Option<usize>,
    },
}

impl PositionsConfig {
    pub fn to_positions(
        &self,
        width: usize,
        height: usize,
        layers: usize,
    ) -> Result<Vec<Position>, ParseError> {
        match self {
            Self::IJK { i, j, k } => {
                if *i >= height || *j >= width || *k >= layers {
                    return Err(ParseError::PositionOutOfBounds {
                        i: *i,
                        j: *j,
                        k: *k,
                    });
                }
                Ok(vec![Position {
                    i: *i,
                    j: *j,
                    k: *k,
                }])
            }
            Self::IJ { i, j } => {
                if *i >= height || *j >= width {
                    return Err(ParseError::PositionOutOfBounds { i: *i, j: *j, k: 0 });
                }
                Ok(vec![Position { i: *i, j: *j, k: 0 }])
            }
            Self::Rect {
                i_min,
                i_max,
                j_min,
                j_max,
                k_min,
                k_max,
            } => {
                if *i_min >= height || *j_min >= width || *k_min >= layers {
                    return Err(ParseError::PositionOutOfBounds {
                        i: *i_min,
                        j: *j_min,
                        k: *k_min,
                    });
                }
                let mut positions = vec![];
                for i in *i_min..=i_max.unwrap_or(height - 1) {
                    for j in *j_min..=j_max.unwrap_or(width - 1) {
                        for k in *k_min..=k_max.unwrap_or(layers - 1) {
                            if i >= height || j >= width || k >= layers {
                                return Err(ParseError::PositionOutOfBounds { i, j, k });
                            }
                            positions.push(Position { i, j, k });
                        }
                    }
                }
                Ok(positions)
            }
            Self::Row { row } => {
                if *row >= height {
                    return Err(ParseError::PositionOutOfBounds {
                        i: *row,
                        j: 0,
                        k: 0,
                    });
                }
                Ok((0..width)
                    .cartesian_product(0..layers)
                    .map(|(j, k)| Position { i: *row, j, k })
                    .collect())
            }
            Self::Column { col } => {
                if *col >= width {
                    return Err(ParseError::PositionOutOfBounds {
                        i: 0,
                        j: *col,
                        k: 0,
                    });
                }
                Ok((0..height)
                    .cartesian_product(0..layers)
                    .map(|(i, k)| Position { i, j: *col, k })
                    .collect())
            }
            Self::Layer { lay } => {
                if *lay >= layers {
                    return Err(ParseError::PositionOutOfBounds {
                        i: 0,
                        j: 0,
                        k: *lay,
                    });
                }
                Ok((0..height)
                    .cartesian_product(0..width)
                    .map(|(i, j)| Position { i, j, k: *lay })
                    .collect())
            }
        }
    }
}

impl From<&Position> for PositionsConfig {
    fn from(pos: &Position) -> Self {
        Self::IJK {
            i: pos.i,
            j: pos.j,
            k: pos.k,
        }
    }
}
