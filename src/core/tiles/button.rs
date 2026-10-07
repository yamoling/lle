use crate::{
    WorldEvent,
    agent::{Agent, AgentId, Colour},
};
/// A pressure-plate-like tile. Standing on it does nothing by itself; taking
/// `Action::Trigger` while standing on it pulses every `Lift` sharing the
/// same `group_id` (see `Lift` and `World::step`). Buttons hold no
/// persistent on/off state - the "pulse" model means every trigger is a
/// fresh one-shot event.
#[derive(Debug, Clone)]
pub struct Button {
    /// If set, only agents of this colour may actuate the button. Several agents
    /// may share a colour, so this authorizes a group rather than one individual.
    authorized_colour: Option<Colour>,
    agent: Option<AgentId>,
    group_id: usize,
}

impl Button {
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

    /// Restrict this button to only be actuable by agents of `colour`.
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

    /// The colour allowed to actuate this button, or `None` when every agent may.
    pub fn authorized_colour(&self) -> Option<Colour> {
        self.authorized_colour
    }

    /// Dispatched by `Tile::actuate`. Returns the lift group this button
    /// belongs to, so `World` knows which `Lift` tiles to notify this tick.
    ///
    /// `colour` is the colour of the agent taking `Action::Trigger` here. The
    /// button only records its occupant's *id*, so the colour to authorize
    /// against comes from the caller.
    pub fn actuate(&mut self, colour: Colour) -> Option<usize> {
        // An unoccupied button cannot be actuated, whatever the colour.
        self.agent()?;

        if self.authorized_colour.is_some_and(|auth| auth != colour) {
            return None;
        }

        Some(self.group_id)
    }
}
