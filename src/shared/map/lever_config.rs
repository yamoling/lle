use crate::shared::Colour;

use super::{ParseError, actuator_token::parse_actuator_token};

/// The configuration of a `Lever` tile (token `H`).
#[derive(Clone, Debug)]
pub struct LeverConfig {
    pub group_id: usize,
    pub authorized_colour: Option<Colour>,
    /// Whether the lever is on at reset. Only the TOML format can set it.
    pub on: bool,
}

impl LeverConfig {
    /// Parses a token of the form `H<group_id>` optionally followed by
    /// `A<colour>`, e.g. `H0` or `H0A1`, with the same grammar as buttons.
    pub fn from_str(value: &str) -> Result<LeverConfig, ParseError> {
        let (group_id, authorized_colour) = parse_actuator_token(value)?;
        Ok(Self {
            group_id,
            authorized_colour,
            on: false,
        })
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
