use crate::{agent::Colour, tiles::Lever};

use super::{ParseError, actuator_token::parse_actuator_token};

/// The configuration of a `Lever` tile (token `H`).
#[derive(Debug)]
pub struct LeverConfig {
    pub group_id: usize,
    pub authorized_colour: Option<Colour>,
}

impl LeverConfig {
    /// Parses a token of the form `H<group_id>` optionally followed by
    /// `A<colour>`, e.g. `H0` or `H0A1`, with the same grammar as buttons.
    pub fn from_str(value: &str) -> Result<LeverConfig, ParseError> {
        let (group_id, authorized_colour) = parse_actuator_token(value)?;
        Ok(Self {
            group_id,
            authorized_colour,
        })
    }

    pub fn build(&self) -> Lever {
        let tile = Lever::new(self.group_id);
        match self.authorized_colour {
            Some(colour) => tile.restricted_to(colour),
            None => tile,
        }
    }
}

impl std::fmt::Display for LeverConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.authorized_colour {
            Some(colour) => write!(f, "H{}A{}", self.group_id, colour),
            None => write!(f, "H{}", self.group_id),
        }
    }
}

impl From<&Lever> for LeverConfig {
    fn from(tile: &Lever) -> Self {
        Self {
            group_id: tile.group_id(),
            authorized_colour: tile.authorized_colour(),
        }
    }
}
