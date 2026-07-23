use std::cell::Cell;

use crate::{
    Position, RuntimeWorldError, WorldEvent,
    agent::{Agent, AgentId},
    tiles::Direction,
};

/// A one-shot elevator tile. Lifts have no persistent on/off state (unlike
/// lasers): pressing a `Button` sharing the same `group_id` "pulses" every
/// matching `Lift` for exactly one tick, relocating whichever agent
/// currently stands on it by one step in its own `direction`.

#[derive(Debug)]
pub struct Lift {
    direction: Direction,
    agent: Option<AgentId>,
    /// If set, only this agent is allowed to step onto the lift.
    /// Not yet enforced: see CLAUDE.md / plan notes on `pre_enter`.
    authorized_agent_id: Cell<Option<AgentId>>,
    group_id: usize,
    /// Set by `notify()` when a same-group `Button` was actuated this tick;
    /// consumed (and reset) by `take_triggered()` during `World::step`'s
    /// second pass.
    triggered: Cell<bool>,
}

impl Lift {
    pub fn new(
        direction: Direction,
        authorized_agent_id: Option<AgentId>,
        group_id: usize,
    ) -> Self {
        Self {
            direction,
            agent: None,
            authorized_agent_id: Cell::new(authorized_agent_id),
            group_id,
            triggered: Cell::new(false),
        }
    }

    pub fn direction(&self) -> Direction {
        self.direction
    }

    pub fn authorized_agent_id(&self) -> Option<AgentId> {
        self.authorized_agent_id.get()
    }

    pub fn group_id(&self) -> usize {
        self.group_id
    }

    /// Record that a same-group button was actuated this tick.
    pub fn notify(&self) {
        self.triggered.set(true);
    }

    /// Read-and-clear the pulse flag. Used by `World` to find which lifts to
    /// act on, and to make sure the pulse never survives past this tick.
    pub fn take_triggered(&self) -> bool {
        self.triggered.replace(false)
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
        self.triggered.set(false);
    }

    pub fn agent(&self) -> Option<AgentId> {
        self.agent
    }
}
