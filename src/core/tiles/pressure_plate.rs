use crate::{
    WorldEvent,
    agent::{Agent, AgentId, Colour},
};

use super::actuator::authorizes;

/// A passive actuator. It pulses every `Lift` sharing its `group_id` at every
/// step in which an authorized agent stands on it after the walk pass. No
/// `Action::Trigger` is needed (nor offered).
#[derive(Debug, Clone)]
pub struct PressurePlate {
    /// If set, only agents of this colour activate the plate.
    authorized_colour: Option<Colour>,
    agent: Option<AgentId>,
    group_id: usize,
}

impl PressurePlate {
    pub fn new(group_id: usize) -> Self {
        Self {
            authorized_colour: None,
            agent: None,
            group_id,
        }
    }

    pub fn group_id(&self) -> usize {
        self.group_id
    }

    /// Restrict this plate to only be activated by agents of `colour`.
    pub fn restricted_to(mut self, colour: Colour) -> Self {
        self.authorized_colour = Some(colour);
        self
    }

    pub fn enter(&mut self, agent: &mut Agent) -> Option<WorldEvent> {
        self.agent = Some(agent.id());
        None
    }

    pub fn leave(&mut self) -> AgentId {
        self.agent.take().expect("No agent to leave")
    }

    pub fn reset(&mut self) {
        self.agent = None;
    }

    pub fn agent(&self) -> Option<AgentId> {
        self.agent
    }

    /// The colour allowed to activate this plate, or `None` when every agent may.
    pub fn authorized_colour(&self) -> Option<Colour> {
        self.authorized_colour
    }

    /// The lift group this plate pulses, given the colour of its occupant
    /// (`None` when the plate is empty).
    pub fn pulsed_group(&self, occupant_colour: Option<Colour>) -> Option<usize> {
        let colour = occupant_colour?;
        authorizes(self.authorized_colour, colour).then_some(self.group_id)
    }
}
