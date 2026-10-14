use crate::{Position, agent::Colour, core::parsing::lever_config::LeverConfig};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TomlLeverConfig {
    pub authorized_colour: Option<Colour>,
    pub position: Position,
    pub group_id: usize,
}

impl TomlLeverConfig {
    pub fn from_lever_config(config: &LeverConfig, position: Position) -> Self {
        Self {
            authorized_colour: config.authorized_colour,
            position,
            group_id: config.group_id,
        }
    }
}

impl From<&TomlLeverConfig> for LeverConfig {
    fn from(val: &TomlLeverConfig) -> Self {
        LeverConfig {
            group_id: val.group_id,
            authorized_colour: val.authorized_colour,
        }
    }
}
