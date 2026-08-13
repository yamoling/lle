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
    /// The decoded model has no position for `agent` at time step `t`, so the trajectory cannot be
    /// reconstructed. This signals an incomplete or malformed SAT model.
    MissingPosition {
        agent: AgentId,
        t: usize,
    },
    /// A parameterized [`SolveMode`](crate::solver::SolveMode) carries a value that has no meaning
    /// for that variant. `variant` is the Rust variant name and `reason` explains what the value
    /// would encode, if anything.
    InvalidModeParameter {
        variant: &'static str,
        value: usize,
        reason: String,
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
            SolverError::MissingPosition { agent, t } => write!(
                f,
                "Incomplete model: agent {agent} has no decoded position at time step {t}."
            ),
            SolverError::InvalidModeParameter {
                variant,
                value,
                reason,
            } => write!(
                f,
                "Invalid parameter {value} for SolveMode::{variant}: {reason}"
            ),
        }
    }
}

impl Error for SolverError {}
