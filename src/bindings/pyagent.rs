use pyo3::prelude::*;

use crate::agent::Agent;

#[pyclass(unsendable, name = "Agent")]
#[derive(Clone)]
pub struct PyAgent {
    pub agent: Agent,
}

#[pymethods]
impl PyAgent {
    #[getter]
    fn num(&self) -> u32 {
        self.agent.num()
    }

    #[getter]
    fn is_dead(&self) -> bool {
        self.agent.is_dead()
    }

    #[getter]
    fn has_arrived(&self) -> bool {
        self.agent.has_arrived()
    }
}
