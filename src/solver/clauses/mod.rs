mod engine;
mod generator;
mod sources;
mod step_buffer;
mod var_pool;

pub type Literal = i32;
pub type Clause = Vec<Literal>;
pub use engine::ClauseEngine;
pub use generator::ClauseGenerator;
pub use step_buffer::StepBuffer;
pub use var_pool::{VarKey, VarPool};
