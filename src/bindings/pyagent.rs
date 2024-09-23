use pyo3::prelude::*;
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pymethods};

use crate::agent::Agent;

/// An agent in the world.
#[gen_stub_pyclass]
#[pyclass(name = "Agent", module = "lle")]
#[derive(Clone)]
pub struct PyAgent {
    pub agent: Agent,
}

#[gen_stub_pymethods]
#[pymethods]
impl PyAgent {
    /// The agent id.
    #[getter]
    fn num(&self) -> usize {
        self.agent.id()
    }

    /// Whether the agent is dead or not.
    #[getter]
    fn is_dead(&self) -> bool {
        self.agent.is_dead()
    }

    /// Whether the agent is alive or not.
    #[getter]
    fn is_alive(&self) -> bool {
        self.agent.is_alive()
    }

    /// Whether the agent has reached an exit or not.
    #[getter]
    fn has_arrived(&self) -> bool {
        self.agent.has_arrived()
    }
}
