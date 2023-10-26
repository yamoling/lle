use std::{error::Error, fmt::Display};

use crate::{agent::AgentId, Action};

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
    InvalidPosition {
        i: i32,
        j: i32,
    },
    InvalidNumberOfActions {
        given: usize,
        expected: usize,
    },
    WorldIsDone,
}

impl Display for RuntimeWorldError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

impl Error for RuntimeWorldError {}
