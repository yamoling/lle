mod generator;
mod lasers;
mod movement;
mod utils;

pub type Literal = i32;
pub type Clause = Vec<Literal>;
pub use generator::ClauseGenerator;

#[cfg(test)]
#[path = "../../unit_tests/test_individual_clauses.rs"]
mod tests;
