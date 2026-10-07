use super::PyPosition;
use crate::{AgentId, BoxId, WorldEvent};
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
    #[pyo3(name = "BOX_DESTROYED")]
    BoxDestroyed,
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
            PyEventType::BoxDestroyed => 4,
        }
    }
}

#[gen_stub_pyclass]
#[derive(Clone)]
#[pyclass(name = "WorldEvent", module = "lle.world", skip_from_py_object)]
pub struct PyWorldEvent {
    /// The kind of event.
    #[pyo3(get)]
    event_type: PyEventType,
    /// The agent concerned by the event, or `None` for events that involve no agent (`BOX_DESTROYED`).
    #[pyo3(get)]
    agent_id: Option<AgentId>,
    /// The box concerned by the event, or `None` for events that involve no box.
    #[pyo3(get)]
    box_id: Option<BoxId>,
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
    #[pyo3(signature = (event_type, agent_id=None, box_id=None, from_position=None, to_position=None))]
    pub fn new(
        event_type: PyEventType,
        agent_id: Option<AgentId>,
        box_id: Option<BoxId>,
        from_position: Option<PyPosition>,
        to_position: Option<PyPosition>,
    ) -> Self {
        Self {
            event_type,
            agent_id,
            box_id,
            from_position,
            to_position,
        }
    }

    fn __str__(&self) -> String {
        match (self.agent_id, self.box_id) {
            (Some(agent_id), _) => format!("{:?}, agent id: {}", self.event_type, agent_id),
            (None, Some(box_id)) => format!("{:?}, box id: {}", self.event_type, box_id),
            (None, None) => format!("{:?}", self.event_type),
        }
    }

    fn __repr__(&self) -> String {
        self.__str__()
    }
}

impl From<&WorldEvent> for PyWorldEvent {
    fn from(val: &WorldEvent) -> Self {
        match val {
            WorldEvent::AgentExit { agent_id } => {
                Self::new(PyEventType::AgentExit, Some(*agent_id), None, None, None)
            }
            WorldEvent::GemCollected { agent_id } => {
                Self::new(PyEventType::GemCollected, Some(*agent_id), None, None, None)
            }
            WorldEvent::AgentDied { agent_id } => {
                Self::new(PyEventType::AgentDied, Some(*agent_id), None, None, None)
            }
            WorldEvent::LiftMoved { agent_id, from, to } => Self::new(
                PyEventType::LiftMoved,
                Some(*agent_id),
                None,
                Some((*from).into()),
                Some((*to).into()),
            ),
            WorldEvent::BoxDestroyed { box_id } => {
                Self::new(PyEventType::BoxDestroyed, None, Some(*box_id), None, None)
            }
        }
    }
}
