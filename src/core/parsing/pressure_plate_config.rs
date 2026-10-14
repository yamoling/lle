use crate::{agent::Colour, tiles::PressurePlate};

use super::{ParseError, actuator_token::parse_actuator_token};

/// The configuration of a `PressurePlate` tile (token `P`).
#[derive(Debug)]
pub struct PressurePlateConfig {
    pub group_id: usize,
    pub authorized_colour: Option<Colour>,
}

impl PressurePlateConfig {
    /// Parses a token of the form `P<group_id>` optionally followed by
    /// `A<colour>`, e.g. `P0` or `P0A1`, with the same grammar as buttons.
    pub fn from_str(value: &str) -> Result<PressurePlateConfig, ParseError> {
        let (group_id, authorized_colour) = parse_actuator_token(value)?;
        Ok(Self {
            group_id,
            authorized_colour,
        })
    }

    pub fn build(&self) -> PressurePlate {
        let tile = PressurePlate::new(self.group_id);
        match self.authorized_colour {
            Some(colour) => tile.restricted_to(colour),
            None => tile,
        }
    }
}

impl std::fmt::Display for PressurePlateConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.authorized_colour {
            Some(colour) => write!(f, "P{}A{}", self.group_id, colour),
            None => write!(f, "P{}", self.group_id),
        }
    }
}

impl From<&PressurePlate> for PressurePlateConfig {
    fn from(tile: &PressurePlate) -> Self {
        Self {
            group_id: tile.group_id(),
            authorized_colour: tile.authorized_colour(),
        }
    }
}
