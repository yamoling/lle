use serde::{Deserialize, Serialize};

use crate::{
    core::parsing::laser_config::LaserConfig,
    tiles::{Direction, LaserId},
    AgentId, Position,
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

impl Into<LaserConfig> for &TomlLaserConfig {
    fn into(self) -> LaserConfig {
        LaserConfig {
            direction: self.direction,
            agent_id: self.agent,
            laser_id: self.laser_id,
        }
    }
}
