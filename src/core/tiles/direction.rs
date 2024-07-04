use std::fmt::Display;

use crate::ParseError;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Direction {
    North,
    East,
    South,
    West,
}

impl Direction {
    pub fn delta(&self) -> (i32, i32) {
        match self {
            Direction::North => (-1, 0),
            Direction::East => (0, 1),
            Direction::South => (1, 0),
            Direction::West => (0, -1),
        }
    }

    pub fn opposite(&self) -> Direction {
        match self {
            Direction::North => Direction::South,
            Direction::East => Direction::West,
            Direction::South => Direction::North,
            Direction::West => Direction::East,
        }
    }

    pub fn to_file_string(&self) -> String {
        match self {
            Direction::North => "N".to_string(),
            Direction::East => "E".to_string(),
            Direction::South => "S".to_string(),
            Direction::West => "W".to_string(),
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
            _ => Err(ParseError::InvalidDirection {
                given: value.into(),
                expected: "{{N, E, S, W, north, east, south, west}}.".into(),
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
        }
    }
}
