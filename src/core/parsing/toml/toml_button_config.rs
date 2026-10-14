use crate::{Position, agent::Colour, core::parsing::button_config::ButtonConfig};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TomlButtonConfig {
    pub authorized_colour: Option<Colour>,
    pub position: Position,
    pub group_id: usize,
}

impl TomlButtonConfig {
    pub fn from_button_config(button: &ButtonConfig, position: Position) -> Self {
        Self {
            authorized_colour: button.authorized_colour,
            position,
            group_id: button.group_id,
        }
    }
}

impl From<&TomlButtonConfig> for ButtonConfig {
    fn from(val: &TomlButtonConfig) -> Self {
        ButtonConfig {
            group_id: val.group_id,
            authorized_colour: val.authorized_colour,
        }
    }
}
