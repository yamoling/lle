use std::{error::Error, fmt::Display, sync::PoisonError};

use pyo3::{PyErr, PyResult, exceptions};

use crate::{
    Action, Position, WorldState,
    agent::AgentId,
    bindings::{InvalidActionError, InvalidWorldStateError},
};

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
    InvalidWorldState {
        reason: String,
        state: WorldState,
    },
    TileNotWalkable,
    MutexPoisoned,
}

impl Display for RuntimeWorldError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

impl Error for RuntimeWorldError {}

impl<T> From<PoisonError<T>> for RuntimeWorldError {
    fn from(_: PoisonError<T>) -> Self {
        RuntimeWorldError::MutexPoisoned
    }
}

impl<T> Into<PyResult<T>> for RuntimeWorldError {
    fn into(self) -> PyResult<T> {
        if let RuntimeWorldError::InvalidAction {
            agent_id,
            available,
            taken,
        } = self
        {
            return Err(InvalidActionError::new_err(format!(
                "Invalid action for agent {agent_id}: available actions: {available:?}, taken action: {taken}",
            )));
        }

        let err = match self {
            RuntimeWorldError::InvalidNumberOfActions { given, expected } => {
                exceptions::PyValueError::new_err(format!(
                    "Invalid number of actions: given {given}, expected {expected}",
                ))
            }
            RuntimeWorldError::OutOfWorldPosition { position } => {
                exceptions::PyIndexError::new_err(format!(
                    "Position {position:?} is out of the world's boundaries",
                ))
            }
            RuntimeWorldError::InvalidNumberOfAgents { given, expected } => {
                InvalidWorldStateError::new_err(format!(
                    "Invalid number of agents: given {given}, expected {expected}",
                ))
            }
            RuntimeWorldError::InvalidNumberOfGems { given, expected } => {
                InvalidWorldStateError::new_err(format!(
                    "Invalid number of gems: given {given}, expected {expected}",
                ))
            }
            RuntimeWorldError::InvalidAgentPosition { position, reason } => {
                InvalidWorldStateError::new_err(format!(
                    "Invalid agent position {position:?}: {reason}",
                ))
            }
            RuntimeWorldError::InvalidAction { .. } => panic!("Already handled above"),
            RuntimeWorldError::InvalidWorldState { reason, state } => {
                InvalidWorldStateError::new_err(format!(
                    "Invalid world state: {reason}. Wrong state: {state:?}"
                ))
            }
            RuntimeWorldError::TileNotWalkable => {
                InvalidWorldStateError::new_err("An agent tried to walk on a non-walkable tile.")
            }
            RuntimeWorldError::MutexPoisoned => {
                panic!("Mutex poisoned ! Check your code for deadlocks or exceptions.")
            }
        };
        Err(err)
    }
}

impl From<RuntimeWorldError> for PyErr {
    fn from(err: RuntimeWorldError) -> Self {
        err.into()
    }
}
