use std::fmt::Display;

use super::{
    laser::{Direction, Laser},
    laser_source::LaserSource,
};

#[derive(Clone)]
pub enum TileType {
    Floor,
    Wall,
    Gem { collected: bool },
    LaserSource(LaserSource),
    Laser(Laser),
    Start { agent_num: u32 },
    Exit,
}

impl TileType {
    pub fn reset(&mut self) {
        match self {
            Self::Gem { collected } => *collected = false,
            Self::Laser(laser) => laser.reset(),
            _ => {}
        }
    }
}

impl From<&str> for TileType {
    fn from(token: &str) -> Self {
        match token.chars().next().unwrap() {
            '.' => Self::Floor,
            'G' => Self::Gem { collected: false },
            'S' => Self::Start {
                agent_num: token[1..].parse().unwrap(),
            },
            '@' => Self::Wall,
            'F' => Self::Exit,
            'L' => {
                let direction = Direction::from(&token[2..]);
                let agent_num = token[1..2].parse().unwrap();
                Self::LaserSource(LaserSource::new(direction, agent_num))
            }
            _ => panic!("Unknown tile type: {}", token),
        }
    }
}

impl Display for TileType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Exit { .. } => write!(f, "F"),
            Self::Floor => write!(f, "."),
            Self::Wall => write!(f, "@"),
            Self::Start { agent_num } => write!(f, "S{}", agent_num),
            Self::Gem { collected } => {
                if !*collected {
                    write!(f, "G")
                } else {
                    write!(f, ".")
                }
            }
            Self::LaserSource(LaserSource {
                agent_num,
                direction,
            }) => match direction {
                Direction::North => write!(f, "S↑{}", agent_num),
                Direction::East => write!(f, "S→{}", agent_num),
                Direction::South => write!(f, "S↓{}", agent_num),
                Direction::West => write!(f, "S←{}", agent_num),
            },
            Self::Laser(laser) => {
                if laser.is_on() {
                    match laser.direction() {
                        Direction::North => write!(f, "↑{}", laser.agent_num()),
                        Direction::East => write!(f, "→{}", laser.agent_num()),
                        Direction::South => write!(f, "↓{}", laser.agent_num()),
                        Direction::West => write!(f, "←{}", laser.agent_num()),
                    }
                } else {
                    write!(f, ".")
                }
            }
        }
    }
}
