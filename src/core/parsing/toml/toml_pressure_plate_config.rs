use crate::{Position, agent::Colour, core::parsing::pressure_plate_config::PressurePlateConfig};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TomlPressurePlateConfig {
    pub authorized_colour: Option<Colour>,
    pub position: Position,
    pub group_id: usize,
}

impl TomlPressurePlateConfig {
    pub fn from_pressure_plate_config(config: &PressurePlateConfig, position: Position) -> Self {
        Self {
            authorized_colour: config.authorized_colour,
            position,
            group_id: config.group_id,
        }
    }
}

impl From<&TomlPressurePlateConfig> for PressurePlateConfig {
    fn from(val: &TomlPressurePlateConfig) -> Self {
        PressurePlateConfig {
            group_id: val.group_id,
            authorized_colour: val.authorized_colour,
        }
    }
}
