mod assumptions;
mod cooperation;
mod gems;
mod generator;
mod lasers;
mod movement;
mod solve_mode;
mod trails;
mod utils;
mod var_pool;
pub type Literal = i32;
pub type Clause = Vec<Literal>;
pub use generator::ClauseGenerator;
pub use solve_mode::SolveMode;
pub use var_pool::{VarKey, VarPool};

#[cfg(test)]
#[path = "../../unit_tests/test_clause_generation.rs"]
mod test_clause_generation;
