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
    /// A trajectory supplied for inspection has an invalid length.
    InvalidTrajectoryLength {
        given: usize,
        max: usize,
    },
    /// A joint action in a trajectory has the wrong number of agent actions.
    InvalidJointActionLength {
        step: usize,
        given: usize,
        expected: usize,
    },
    /// A trajectory action cannot be applied to the current position.
    InvalidActionInTrajectory {
        step: usize,
    },
    /// The movement layer did not materialize the expected position literal for a trajectory.
    MissingTrajectoryLiteral {
        agent_id: AgentId,
        pos: Position,
        t: usize,
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
            SolverError::InvalidTrajectoryLength { given, max } => write!(
                f,
                "Invalid trajectory length: got {given}, expected at most or exactly {max} for this operation."
            ),
            SolverError::InvalidJointActionLength {
                step,
                given,
                expected,
            } => write!(
                f,
                "Invalid joint action at step {step}: got {given} actions, expected {expected}."
            ),
            SolverError::InvalidActionInTrajectory { step } => {
                write!(f, "Invalid action in trajectory at step {step}.")
            }
            SolverError::MissingTrajectoryLiteral { agent_id, pos, t } => write!(
                f,
                "Trajectory position literal was not materialized: agent {agent_id} at {pos:?} at time {t}."
            ),
        }
    }
}

impl Error for SolverError {}
