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

impl TryFrom<&str> for Direction {
    type Error = &'static str;
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "N" => Ok(Direction::North),
            "E" => Ok(Direction::East),
            "S" => Ok(Direction::South),
            "W" => Ok(Direction::West),
            _ => Err("Invalid direction"),
        }
    }
}
