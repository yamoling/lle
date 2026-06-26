mod assumptions;
mod cooperation;
mod dependency_shapes;
mod engine;
mod gems;
mod generator;
mod lasers;
mod movement;
mod solve_mode;
mod sources;
mod step_buffer;
mod utils;
mod var_pool;

pub type Literal = i32;
pub type Clause = Vec<Literal>;
pub use generator::ClauseGenerator;
pub use solve_mode::SolveMode;
pub use step_buffer::StepBuffer;
pub use var_pool::{VarKey, VarPool};

#[cfg(test)]
#[path = "../../unit_tests/test_clause_generation.rs"]
mod test_clause_generation;
