use std::{error::Error, fmt::Display};

use crate::{AgentId, Position};

use super::clauses::VarKey;

#[derive(Debug)]
pub enum SolverError {
    VariableNotCreated {
        var: VarKey,
    },
    InvalidAssumption {
        var: VarKey,
        reason: String,
    },
    InvalidTrajectory {
        prev_pos: Position,
        current_pos: Position,
        agent: AgentId,
        index: usize,
    },
}

impl Display for SolverError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SolverError::VariableNotCreated { var } => write!(f, "Variable not created: {var:?}"),
            SolverError::InvalidAssumption { var, reason } => {
                write!(f, "Invalid assumption for {var:?}: {reason}")
            }
            SolverError::InvalidTrajectory {
                prev_pos,
                current_pos: next_pos,
                agent,
                index,
            } => {
                let (di, dj) = (next_pos.i - prev_pos.i, next_pos.j - prev_pos.j);
                let distance = di + dj;
                write!(
                    f,
                    "Invalid trajectory at index {index}: agent {agent} goes from {prev_pos:?} to {next_pos:?} (i.e. a distance of {distance} tiles), which does not match any possible action."
                )
            }
        }
    }
}

impl Error for SolverError {}
