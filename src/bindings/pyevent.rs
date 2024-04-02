use crate::{AgentId, WorldEvent};
use pyo3::prelude::*;

#[derive(Clone, Debug, PartialEq)]
#[pyclass(name = "EventType", module = "lle")]
pub enum PyEventType {
    #[pyo3(name = "AGENT_EXIT")]
    AgentExit,
    #[pyo3(name = "GEM_COLLECTED")]
    GemCollected,
    #[pyo3(name = "AGENT_DIED")]
    AgentDied,
}

#[derive(Clone)]
#[pyclass(name = "WorldEvent", module = "lle")]
pub struct PyWorldEvent {
    #[pyo3(get)]
    event_type: PyEventType,
    // pos: Position,
    #[pyo3(get)]
    agent_id: AgentId,
}

impl PyWorldEvent {
    pub fn new(event_type: PyEventType, agent_id: AgentId) -> Self {
        Self {
            event_type,
            agent_id,
        }
    }
}

#[pymethods]
impl PyWorldEvent {
    fn __str__(&self) -> String {
        format!("{:?}, agent id: {}", self.event_type, self.agent_id)
    }

    fn __repr__(&self) -> String {
        self.__str__()
    }
}

impl From<&WorldEvent> for PyWorldEvent {
    fn from(val: &WorldEvent) -> Self {
        let (event_type, agent_id) = match val {
            WorldEvent::AgentExit { agent_id } => (PyEventType::AgentExit, agent_id),
            WorldEvent::GemCollected { agent_id } => (PyEventType::GemCollected, agent_id),
            WorldEvent::AgentDied { agent_id } => (PyEventType::AgentDied, agent_id),
        };
        PyWorldEvent {
            agent_id: *agent_id,
            event_type,
        }
    }
}
