use crate::{
    agent::{Agent, AgentId},
    rendering::{TileVisitor, VisitorData},
    ParseError, RuntimeWorldError, WorldEvent,
};
use core::panic;
use std::rc::Rc;

use super::{Gem, Laser, LaserBeam, LaserSource, Start, Void};

pub enum Tile {
    Gem(Gem),
    Floor { agent: Option<AgentId> },
    Wall,
    Void(Void),
    Start(Start),
    Exit { agent: Option<AgentId> },
    Laser(Laser),
    LaserSource(LaserSource),
}

impl Tile {
    pub fn pre_enter(&mut self, agent: &Agent) -> Result<(), RuntimeWorldError> {
        match self {
            Self::Laser(laser) => laser.pre_enter(agent),
            Self::Wall | Self::LaserSource(_) => return Err(RuntimeWorldError::TileNotWalkable),
            _ => Ok(()),
        }
    }

    pub fn enter(&mut self, agent: &mut Agent) -> Option<WorldEvent> {
        match self {
            Self::Wall | Self::LaserSource(_) => panic!("Cannot enter a wall or a laser source"),
            Self::Exit { agent: slot } => {
                *slot = Some(agent.id());
                if !agent.has_arrived() {
                    agent.arrive();
                    return Some(WorldEvent::AgentExit {
                        agent_id: agent.id(),
                    });
                }
                None
            }
            Self::Floor { agent: slot } => {
                *slot = Some(agent.id());
                None
            }
            Self::Start(start) => start.enter(agent),
            Self::Void(void) => void.enter(agent),
            Self::Laser(laser) => laser.enter(agent),
            Self::Gem(gem) => gem.enter(agent),
        }
    }

    pub fn leave(&mut self) -> AgentId {
        match self {
            Self::Wall | Self::LaserSource(_) => panic!("Cannot leave a wall or a laser source"),
            Self::Exit { agent: slot } => slot.take().expect("No agent to leave"),
            Self::Floor { agent: slot } => slot.take().expect("No agent to leave"),
            Self::Start(start) => start.leave(),
            Self::Void(void) => void.leave(),
            Self::Laser(laser) => laser.leave(),
            Self::Gem(gem) => gem.leave(),
        }
    }

    pub fn is_waklable(&self) -> bool {
        match self {
            Self::Gem(_) => true,
            Self::LaserSource(_) => false,
            Self::Wall => false,
            Self::Floor { .. } => true,
            Self::Void { .. } => true,
            Self::Start(_) => true,
            Self::Exit { .. } => true,
            Self::Laser(_) => true,
        }
    }

    pub fn reset(&mut self) {
        match self {
            Self::Gem(gem) => gem.reset(),
            Self::LaserSource(..) | Self::Wall => {}
            Self::Exit { agent } => *agent = None,
            Self::Floor { agent } => *agent = None,
            Self::Start(start) => start.reset(),
            Self::Void(void) => void.reset(),
            Self::Laser(laser) => laser.reset(),
        }
    }

    pub fn agent(&self) -> Option<AgentId> {
        match self {
            Self::Gem(gem) => gem.agent(),
            Self::Wall | Self::LaserSource(_) => None,
            Self::Exit { agent } => *agent,
            Self::Floor { agent } => *agent,
            Self::Start(start) => start.agent(),
            Self::Void(void) => void.agent(),
            Self::Laser(laser) => laser.agent(),
        }
    }

    pub fn is_occupied(&self) -> bool {
        self.agent().is_some()
    }

    pub fn to_file_string(&self) -> String {
        match self {
            Self::Start(start) => return format!("S{}", start.start_agent_id()),
            Self::Laser(laser) => return laser.wrapped().to_file_string(),
            Self::LaserSource(source) => {
                return format!(
                    "L{}{}",
                    source.agent_id(),
                    source.direction().to_file_string()
                );
            }
            _ => {}
        };
        match self {
            Self::Gem(..) => "G",
            Self::Wall => "@",
            Self::Exit { .. } => "X",
            Self::Floor { .. } => ".",
            Self::Void(..) => "V",
            Self::Start(..) | Self::Laser(..) | Self::LaserSource(..) => {
                panic!("Should have been handled before")
            }
        }
        .to_string()
    }

    pub fn accept(&self, visitor: &dyn TileVisitor, data: &mut VisitorData) {
        match self {
            Self::Gem(gem) => visitor.visit_gem(gem, data),
            Self::Laser(laser) => visitor.visit_laser(laser, data),
            Self::LaserSource(source) => visitor.visit_laser_source(source, data),
            _ => {} // Nothing to do
        };
    }

    pub fn try_from_str(value: &str, row: usize, col: usize) -> Result<Self, ParseError> {
        match value.to_uppercase().chars().next().unwrap() {
            'G' => Ok(Self::Gem(Gem::default())),
            '.' => Ok(Self::Floor { agent: None }),
            '@' => Ok(Self::Wall),
            'X' => Ok(Self::Exit { agent: None }),
            'V' => Ok(Self::Void(Void::default())),
            'S' => {
                let agent_id = value[1..].parse().map_err(|_| ParseError::InvalidAgentId {
                    given_agent_id: value[1..].into(),
                })?;
                Ok(Self::Start(Start::new(agent_id)))
            }
            'L' => Ok(Self::LaserSource(LaserSource::new(Rc::new(
                LaserBeam::new(0, 0, super::Direction::East, 0),
            )))),
            _ => Err(ParseError::InvalidTile {
                tile_str: value.into(),
                line: row,
                col,
            }),
        }
    }
}

#[cfg(test)]
#[path = "../../unit_tests/test_tile.rs"]
mod tests;
