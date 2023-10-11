use pyo3::prelude::*;

use crate::agent::Agent;

#[pyclass(name = "Agent")]
#[derive(Clone)]
pub struct PyAgent {
    pub agent: Agent,
}

#[pymethods]
impl PyAgent {
    #[getter]
    fn num(&self) -> usize {
        self.agent.id()
    }

    #[getter]
    fn is_dead(&self) -> bool {
        self.agent.is_dead()
    }

    #[getter]
    fn is_alive(&self) -> bool {
        self.agent.is_alive()
    }

    #[getter]
    fn has_arrived(&self) -> bool {
        self.agent.has_arrived()
    }
}
