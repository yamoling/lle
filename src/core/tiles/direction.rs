use serde::{Deserialize, Serialize};
use std::fmt::Display;
use strum::EnumIter;

use crate::ParseError;

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, EnumIter)]
pub enum Direction {
    #[serde(alias = "N", alias = "north", alias = "n")]
    North,
    #[serde(alias = "E", alias = "east", alias = "e")]
    East,
    #[serde(alias = "S", alias = "south", alias = "s")]
    South,
    #[serde(alias = "W", alias = "west", alias = "w")]
    West,
    #[serde(alias = "U", alias = "up", alias = "u")]
    Up,
    #[serde(alias = "D", alias = "down", alias = "d")]
    Down,
}

impl Direction {
    pub fn delta(&self) -> (i32, i32, i32) {
        match self {
            Direction::North => (-1, 0, 0),
            Direction::East => (0, 1, 0),
            Direction::South => (1, 0, 0),
            Direction::West => (0, -1, 0),
            Direction::Up => (0, 0, 1),
            Direction::Down => (0, 0, -1),
        }
    }

    pub fn opposite(&self) -> Direction {
        match self {
            Direction::North => Direction::South,
            Direction::East => Direction::West,
            Direction::South => Direction::North,
            Direction::West => Direction::East,
            Direction::Up => Direction::Down,
            Direction::Down => Direction::Up,
        }
    }

    pub fn to_file_string(&self) -> String {
        match self {
            Direction::North => "N".to_string(),
            Direction::East => "E".to_string(),
            Direction::South => "S".to_string(),
            Direction::West => "W".to_string(),
            Direction::Up => "U".to_string(),
            Direction::Down => "D".to_string(),
        }
    }
}

impl Display for Direction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

impl TryFrom<&str> for Direction {
    type Error = ParseError;
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value.to_lowercase().as_str() {
            "n" | "north" => Ok(Direction::North),
            "e" | "east" => Ok(Direction::East),
            "s" | "south" => Ok(Direction::South),
            "w" | "west" => Ok(Direction::West),
            "u" | "up" => Ok(Direction::Up),
            "d" | "down" => Ok(Direction::Down),
            _ => Err(ParseError::InvalidDirection {
                given: value.into(),
                expected: "{{N, E, S, W, U, D, north, east, south, west, up, down}}.".into(),
            }),
        }
    }
}

impl TryFrom<char> for Direction {
    type Error = ParseError;
    fn try_from(value: char) -> Result<Self, Self::Error> {
        Direction::try_from(value.to_string().as_str())
    }
}

impl Into<&str> for Direction {
    fn into(self) -> &'static str {
        match self {
            Direction::North => "N",
            Direction::East => "E",
            Direction::South => "S",
            Direction::West => "W",
            Direction::Up => "U",
            Direction::Down => "D",
        }
    }
}

/// A direction restricted to the horizontal plane (no floor change).
///
/// Kept as a distinct type (rather than reusing [`Direction`]) so that code which is only ever
/// meant to deal with the 4 cardinal directions - lasers, rendering, the single-floor solver -
/// cannot silently be handed `Up`/`Down`: the compiler rejects it instead.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, EnumIter)]
pub enum CardinalDirection {
    #[serde(alias = "N", alias = "north", alias = "n")]
    North,
    #[serde(alias = "E", alias = "east", alias = "e")]
    East,
    #[serde(alias = "S", alias = "south", alias = "s")]
    South,
    #[serde(alias = "W", alias = "west", alias = "w")]
    West,
}

impl CardinalDirection {
    pub fn delta(&self) -> (i32, i32) {
        match self {
            CardinalDirection::North => (-1, 0),
            CardinalDirection::East => (0, 1),
            CardinalDirection::South => (1, 0),
            CardinalDirection::West => (0, -1),
        }
    }

    pub fn opposite(&self) -> CardinalDirection {
        match self {
            CardinalDirection::North => CardinalDirection::South,
            CardinalDirection::East => CardinalDirection::West,
            CardinalDirection::South => CardinalDirection::North,
            CardinalDirection::West => CardinalDirection::East,
        }
    }

    pub fn to_file_string(&self) -> String {
        match self {
            CardinalDirection::North => "N".to_string(),
            CardinalDirection::East => "E".to_string(),
            CardinalDirection::South => "S".to_string(),
            CardinalDirection::West => "W".to_string(),
        }
    }
}

impl Display for CardinalDirection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

impl TryFrom<&str> for CardinalDirection {
    type Error = ParseError;
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value.to_lowercase().as_str() {
            "n" | "north" => Ok(CardinalDirection::North),
            "e" | "east" => Ok(CardinalDirection::East),
            "s" | "south" => Ok(CardinalDirection::South),
            "w" | "west" => Ok(CardinalDirection::West),
            _ => Err(ParseError::InvalidDirection {
                given: value.into(),
                expected: "{{N, E, S, W, north, east, south, west}}.".into(),
            }),
        }
    }
}

impl TryFrom<char> for CardinalDirection {
    type Error = ParseError;
    fn try_from(value: char) -> Result<Self, Self::Error> {
        CardinalDirection::try_from(value.to_string().as_str())
    }
}

impl Into<&str> for CardinalDirection {
    fn into(self) -> &'static str {
        match self {
            CardinalDirection::North => "N",
            CardinalDirection::East => "E",
            CardinalDirection::South => "S",
            CardinalDirection::West => "W",
        }
    }
}

impl From<CardinalDirection> for Direction {
    fn from(value: CardinalDirection) -> Self {
        match value {
            CardinalDirection::North => Direction::North,
            CardinalDirection::East => Direction::East,
            CardinalDirection::South => Direction::South,
            CardinalDirection::West => Direction::West,
        }
    }
}

/// A direction restricted to the vertical axis (floor changes only).
///
/// Kept as a distinct type (rather than reusing [`Direction`]) so that a `Lift` -
/// which can only ever move an agent between layers, never sideways - cannot
/// silently be handed `North`/`East`/`South`/`West`: the compiler rejects it
/// instead.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, EnumIter)]
pub enum VerticalDirection {
    #[serde(alias = "U", alias = "up", alias = "u")]
    Up,
    #[serde(alias = "D", alias = "down", alias = "d")]
    Down,
}

impl VerticalDirection {
    pub fn delta(&self) -> (i32, i32, i32) {
        match self {
            VerticalDirection::Up => (0, 0, 1),
            VerticalDirection::Down => (0, 0, -1),
        }
    }

    pub fn opposite(&self) -> VerticalDirection {
        match self {
            VerticalDirection::Up => VerticalDirection::Down,
            VerticalDirection::Down => VerticalDirection::Up,
        }
    }

    pub fn to_file_string(&self) -> String {
        match self {
            VerticalDirection::Up => "U".to_string(),
            VerticalDirection::Down => "D".to_string(),
        }
    }
}

impl Display for VerticalDirection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

impl TryFrom<&str> for VerticalDirection {
    type Error = ParseError;
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value.to_lowercase().as_str() {
            "u" | "up" => Ok(VerticalDirection::Up),
            "d" | "down" => Ok(VerticalDirection::Down),
            _ => Err(ParseError::InvalidDirection {
                given: value.into(),
                expected: "{{U, D, up, down}} (a vertical direction).".into(),
            }),
        }
    }
}

impl TryFrom<char> for VerticalDirection {
    type Error = ParseError;
    fn try_from(value: char) -> Result<Self, Self::Error> {
        VerticalDirection::try_from(value.to_string().as_str())
    }
}

impl Into<&str> for VerticalDirection {
    fn into(self) -> &'static str {
        match self {
            VerticalDirection::Up => "U",
            VerticalDirection::Down => "D",
        }
    }
}

impl From<VerticalDirection> for Direction {
    fn from(value: VerticalDirection) -> Self {
        match value {
            VerticalDirection::Up => Direction::Up,
            VerticalDirection::Down => Direction::Down,
        }
    }
}
