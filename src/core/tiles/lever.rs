use crate::{
    WorldEvent,
    agent::{Agent, AgentId, Colour},
};

use super::actuator::authorizes;

/// A toggle actuator. Taking `Action::Trigger` while standing on it flips it on
/// or off. While it is on, it pulses every `Lift` sharing its `group_id` at
/// every step, including the step it was switched on, whether or not an agent
/// still stands on it. Switching it off stops the pulses in that same step.
///
/// Unlike a `Button`, a lever holds persistent state: its on/off flag is part
/// of the `WorldState`. A lever starts in its initial state, off by default
/// (`on = true` in a TOML map switches it on at every reset).
#[derive(Debug, Clone)]
pub struct Lever {
    /// If set, only agents of this colour may toggle the lever.
    authorized_colour: Option<Colour>,
    agent: Option<AgentId>,
    group_id: usize,
    on: bool,
    /// The state the lever is reset to.
    initially_on: bool,
}

impl Lever {
    pub fn new(group_id: usize) -> Self {
        Self {
            authorized_colour: None,
            agent: None,
            group_id,
            on: false,
            initially_on: false,
        }
    }

    /// Set the state the lever starts in, and is reset to.
    pub fn initially_on(mut self, on: bool) -> Self {
        self.initially_on = on;
        self.on = on;
        self
    }

    /// Whether the lever is on after a reset.
    pub fn is_initially_on(&self) -> bool {
        self.initially_on
    }

    pub fn group_id(&self) -> usize {
        self.group_id
    }

    /// Restrict this lever to only be toggled by agents of `colour`.
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

    /// Removes the occupant and restores the initial on/off state.
    pub fn reset(&mut self) {
        self.agent = None;
        self.on = self.initially_on;
    }

    pub fn agent(&self) -> Option<AgentId> {
        self.agent
    }

    /// The colour allowed to toggle this lever, or `None` when every agent may.
    pub fn authorized_colour(&self) -> Option<Colour> {
        self.authorized_colour
    }

    pub fn is_on(&self) -> bool {
        self.on
    }

    pub fn set_on(&mut self, on: bool) {
        self.on = on;
    }

    /// The lift group this lever pulses this step, if it is on.
    pub fn pulsed_group(&self) -> Option<usize> {
        self.on.then_some(self.group_id)
    }

    /// Dispatched by `Tile::actuate`. Toggles the lever if it is occupied and
    /// `colour` (the colour of the agent taking `Action::Trigger`) is authorized.
    /// The pulse itself is read afterwards with `pulsed_group`.
    pub fn actuate(&mut self, colour: Colour) {
        if self.agent.is_some() && authorizes(self.authorized_colour, colour) {
            self.on = !self.on;
        }
    }
}
