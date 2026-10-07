use serde::{Deserialize, Serialize};

use crate::{
    Position, agent::Colour, core::parsing::lift_config::LiftConfig, tiles::VerticalDirection,
};

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TomlLiftConfig {
    pub direction: VerticalDirection,
    pub authorized_colour: Option<Colour>,
    pub position: Position,
    pub group_id: usize,
}

impl TomlLiftConfig {
    pub fn from_lift_config(lift: &LiftConfig, position: Position) -> Self {
        Self {
            direction: lift.direction,
            authorized_colour: lift.authorized_colour,
            position,
            group_id: lift.group_id,
        }
    }
}

impl Into<LiftConfig> for &TomlLiftConfig {
    fn into(self) -> LiftConfig {
        LiftConfig {
            direction: self.direction,
            authorized_colour: self.authorized_colour,
            group_id: self.group_id,
        }
    }
}
