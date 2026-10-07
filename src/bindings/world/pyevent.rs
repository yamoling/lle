use super::PyPosition;
use crate::{AgentId, WorldEvent};
use pyo3::prelude::*;
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pyclass_enum, gen_stub_pymethods};

/// An enumeration of the events that can occur in the world.
#[gen_stub_pyclass_enum]
#[pyclass(name = "EventType", module = "lle.world", eq, eq_int, from_py_object)]
#[derive(Clone, Debug, PartialEq)]
pub enum PyEventType {
    #[pyo3(name = "AGENT_EXIT")]
    AgentExit,
    #[pyo3(name = "GEM_COLLECTED")]
    GemCollected,
    #[pyo3(name = "AGENT_DIED")]
    AgentDied,
    #[pyo3(name = "LIFT_MOVED")]
    LiftMoved,
}

#[gen_stub_pymethods]
#[pymethods]
impl PyEventType {
    fn __repr__(&self) -> String {
        format!("{self:?}")
    }

    fn __hash__(&self) -> usize {
        match self {
            PyEventType::AgentExit => 0,
            PyEventType::GemCollected => 1,
            PyEventType::AgentDied => 2,
            PyEventType::LiftMoved => 3,
        }
    }
}

#[gen_stub_pyclass]
#[derive(Clone)]
#[pyclass(name = "WorldEvent", module = "lle.world", skip_from_py_object)]
pub struct PyWorldEvent {
    #[pyo3(get)]
    event_type: PyEventType,
    #[pyo3(get)]
    agent_id: AgentId,
    /// The position the agent was relocated from. Only set for `LIFT_MOVED` events.
    #[pyo3(get)]
    from_position: Option<PyPosition>,
    /// The position the agent was relocated to. Only set for `LIFT_MOVED` events.
    #[pyo3(get)]
    to_position: Option<PyPosition>,
}

#[gen_stub_pymethods]
#[pymethods]
impl PyWorldEvent {
    #[new]
    #[pyo3(signature = (event_type, agent_id, from_position=None, to_position=None))]
    pub fn new(
        event_type: PyEventType,
        agent_id: AgentId,
        from_position: Option<PyPosition>,
        to_position: Option<PyPosition>,
    ) -> Self {
        Self {
            event_type,
            agent_id,
            from_position,
            to_position,
        }
    }
    fn __str__(&self) -> String {
        format!("{:?}, agent id: {}", self.event_type, self.agent_id)
    }

    fn __repr__(&self) -> String {
        self.__str__()
    }
}

impl From<&WorldEvent> for PyWorldEvent {
    fn from(val: &WorldEvent) -> Self {
        let (event_type, agent_id, from_position, to_position) = match val {
            WorldEvent::AgentExit { agent_id } => (PyEventType::AgentExit, agent_id, None, None),
            WorldEvent::GemCollected { agent_id } => {
                (PyEventType::GemCollected, agent_id, None, None)
            }
            WorldEvent::AgentDied { agent_id } => (PyEventType::AgentDied, agent_id, None, None),
            WorldEvent::LiftMoved { agent_id, from, to } => (
                PyEventType::LiftMoved,
                agent_id,
                Some((*from).into()),
                Some((*to).into()),
            ),
        };
        PyWorldEvent {
            agent_id: *agent_id,
            event_type,
            from_position,
            to_position,
        }
    }
}
