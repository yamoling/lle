use std::{error::Error, fmt::Display};

use crate::shared::{AgentId, ParseError};

/// Every failure of the continuous engine: map loading, physics parameters, actions and states.
#[derive(Debug)]
pub enum ContinuousError {
    /// The map could not be loaded or validated.
    Parse(ParseError),
    /// The map uses features the continuous engine does not support, listed in the order
    /// `boxes, lifts, buttons, levers, pressure_plates, layers`.
    UnsupportedFeature { features: Vec<String> },
    /// The physics parameters are invalid (see [`super::PhysicsParams::validate`]).
    InvalidParams { reason: String },
    /// The number of actions differs from the number of agents.
    InvalidNumberOfActions { given: usize, expected: usize },
    /// The action of an agent is not finite.
    InvalidAction { agent_id: AgentId, action: [f64; 2] },
    /// The state given to `set_state` does not fit the world or is physically impossible.
    InvalidState { reason: String },
}

impl From<ParseError> for ContinuousError {
    fn from(error: ParseError) -> Self {
        ContinuousError::Parse(error)
    }
}

impl Display for ContinuousError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

impl Error for ContinuousError {}

#[cfg(test)]
#[path = "../unit_tests/test_continuous_errors.rs"]
mod tests;
