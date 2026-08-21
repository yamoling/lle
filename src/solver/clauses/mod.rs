mod delta_stream;
mod engine;
mod generator;
mod layout_facts;
mod mode_requirements;
mod parameterized_step_buffer;
mod step_buffer;
mod var_pool;

pub type Literal = i32;
pub type Clause = Vec<Literal>;
pub use delta_stream::DeltaStream;
pub use engine::ClauseEngine;
pub use generator::ClauseGenerator;
pub use parameterized_step_buffer::ParameterizedStepBuffer;
pub use step_buffer::StepBuffer;
pub use var_pool::{VarKey, VarPool};
