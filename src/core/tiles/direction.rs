use std::fmt::Display;

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
}

impl Display for Direction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

impl TryFrom<&str> for Direction {
    type Error = String;
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value.to_lowercase().as_str() {
            "n" | "north" => Ok(Direction::North),
            "e" | "east" => Ok(Direction::East),
            "s" | "south" => Ok(Direction::South),
            "w" | "west" => Ok(Direction::West),
            other => Err(format!(
                "Invalid direction: {other}. Expected one of {{N, E, S, W, north, east, south, west}}."
            )),
        }
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
