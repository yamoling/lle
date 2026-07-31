use serde::{Deserialize, Serialize};

use crate::{AgentId, Position, core::parsing::lift_config::LiftConfig, tiles::VerticalDirection};

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TomlLiftConfig {
    pub direction: VerticalDirection,
    pub authorized_agent_id: Option<AgentId>,
    pub position: Position,
    pub group_id: usize,
}

impl TomlLiftConfig {
    pub fn from_lift_config(lift: &LiftConfig, position: Position) -> Self {
        Self {
            direction: lift.direction,
            authorized_agent_id: lift.authorized_agent_id,
            position,
            group_id: lift.group_id,
        }
    }
}

impl Into<LiftConfig> for &TomlLiftConfig {
    fn into(self) -> LiftConfig {
        LiftConfig {
            direction: self.direction,
            authorized_agent_id: self.authorized_agent_id,
            group_id: self.group_id,
        }
    }
}
