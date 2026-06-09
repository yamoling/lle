use std::{error::Error, fmt::Display};

use super::clauses::VarKey;

#[derive(Debug)]
pub enum SolverError {
    VariableNotCreated { var: VarKey },
    InvalidAssumption { var: VarKey, reason: String },
}

impl Display for SolverError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SolverError::VariableNotCreated { var } => write!(f, "Variable not created: {var:?}"),
            SolverError::InvalidAssumption { var, reason } => {
                write!(f, "Invalid assumption for {var:?}: {reason}")
            }
        }
    }
}

impl Error for SolverError {}
