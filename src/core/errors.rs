use std::{error::Error, fmt::Display};

use crate::{agent::AgentId, Action, Position};

#[derive(Debug)]
pub enum RuntimeWorldError {
    InvalidAction {
        agent_id: AgentId,
        available: Vec<Action>,
        taken: Action,
    },
    InvalidNumberOfGems {
        given: usize,
        expected: usize,
    },
    InvalidNumberOfAgents {
        given: usize,
        expected: usize,
    },
    InvalidAgentPosition {
        position: Position,
        reason: String,
    },
    OutOfWorldPosition {
        position: Position,
    },
    InvalidNumberOfActions {
        given: usize,
        expected: usize,
    },
}

impl Display for RuntimeWorldError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

impl Error for RuntimeWorldError {}
