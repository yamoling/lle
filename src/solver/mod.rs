//! Pure-Rust port of the SAT constraint generation used by `lle.solver.solve`.
//!
//! This module reproduces (and replaces) the Python `ConstraintContext`,
//! `VariableFactory` and `Initialization/Movement/Laser/Objective` constraint
//! generators. The SAT solving itself (Minisat22 via `pysat`) remains in Python;
//! this module only produces the CNF clauses and decodes solver models back into
//! plans.

mod clauses;
mod context;
pub mod errors;
pub mod position_set;
mod var_pool;

pub use clauses::{Clause, ClauseGenerator};
// pub use context::ConstraintContext;
pub use var_pool::VarKey;
