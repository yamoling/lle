use crate::{
    Position, RuntimeWorldError, WorldEvent,
    agent::{Agent, AgentId, Colour},
    tiles::VerticalDirection,
};

/// A one-shot elevator tile. Lifts have no persistent on/off state (unlike
/// lasers): pressing a `Button` sharing the same `group_id` "pulses" every
/// matching `Lift` for exactly one tick, relocating whichever agent
/// currently stands on it by one step in its own `direction`.

#[derive(Debug)]
pub struct Lift {
    direction: VerticalDirection,
    agent: Option<AgentId>,
    /// If set, only agents of this colour are relocated when the lift is pulsed;
    /// riders of any other colour are left in place (enforced in
    /// `World::resolve_lift_moves`). Several agents may share a colour, so this
    /// authorizes a group rather than one individual.
    authorized_colour: Option<Colour>,
    group_id: usize,
}

impl Lift {
    pub fn new(
        direction: VerticalDirection,
        authorized_colour: Option<Colour>,
        group_id: usize,
    ) -> Self {
        Self {
            direction,
            agent: None,
            authorized_colour,
            group_id,
        }
    }

    pub fn direction(&self) -> VerticalDirection {
        self.direction
    }

    /// The colour allowed to ride this lift, or `None` when every agent may.
    pub fn authorized_colour(&self) -> Option<Colour> {
        self.authorized_colour
    }

    pub fn group_id(&self) -> usize {
        self.group_id
    }

    /// Where this lift would send `from`'s occupant, given its `direction`.
    pub fn destination(&self, from: Position) -> Result<Position, RuntimeWorldError> {
        from + self.direction
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
}
