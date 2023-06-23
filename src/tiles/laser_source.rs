use std::fmt::Display;

use super::laser::Direction;

#[derive(Clone, Copy)]
pub struct LaserSource {
    pub direction: Direction,
    pub agent_num: u32,
}

impl LaserSource {
    pub fn new(direction: Direction, agent_num: u32) -> Self {
        Self {
            direction,
            agent_num,
        }
    }

    pub fn direction(&self) -> Direction {
        self.direction
    }

    pub fn agent_num(&self) -> u32 {
        self.agent_num
    }
}

impl Display for LaserSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.direction {
            Direction::North => write!(f, "S↑{}", self.agent_num),
            Direction::East => write!(f, "S→{}", self.agent_num),
            Direction::South => write!(f, "S↓{}", self.agent_num),
            Direction::West => write!(f, "S←{}", self.agent_num),
        }
    }
}
