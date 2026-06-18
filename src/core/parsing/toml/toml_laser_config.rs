use serde::{Deserialize, Serialize};

use crate::{
    AgentId, Position,
    core::parsing::laser_config::LaserConfig,
    tiles::{Direction, LaserId},
};

#[derive(Debug, Deserialize, Serialize)]
pub struct TomlLaserConfig {
    pub direction: Direction,
    pub agent: AgentId,
    pub position: Position,
    pub laser_id: LaserId,
}

impl TomlLaserConfig {
    pub fn from_laser_config(laser: &LaserConfig, position: Position) -> Self {
        Self {
            direction: laser.direction,
            agent: laser.agent_id,
            position,
            laser_id: laser.laser_id,
        }
    }
}

impl From<&TomlLaserConfig> for LaserConfig {
    fn from(val: &TomlLaserConfig) -> Self {
        LaserConfig {
            direction: val.direction,
            agent_id: val.agent,
            laser_id: val.laser_id,
        }
    }
}
