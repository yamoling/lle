use std::rc::Rc;

use crate::{agent::AgentId, tiles::Direction};

use super::LaserBeam;

pub type LaserId = usize;

#[derive(Clone, Debug)]
pub struct LaserSource {
    beam: Rc<LaserBeam>,
}

impl LaserSource {
    pub fn new(beam: Rc<LaserBeam>) -> Self {
        Self { beam }
    }

    pub fn is_enabled(&self) -> bool {
        self.beam.is_enabled()
    }

    pub fn agent_id(&self) -> AgentId {
        self.beam.agent_id()
    }

    /// The beam's colour. Every agent of this colour may block and cross the beam.
    ///
    /// **Not implemented yet** — see `.agents/plans/agent-colour-id.md` §3.1.
    pub fn colour(&self) -> AgentId {
        todo!("LaserSource::colour: see .agents/plans/agent-colour-id.md §3.1")
    }

    pub fn direction(&self) -> Direction {
        self.beam.direction()
    }

    pub fn laser_id(&self) -> LaserId {
        self.beam.laser_id()
    }

    pub fn enable(&self) {
        self.beam.enable()
    }

    pub fn disable(&self) {
        self.beam.disable()
    }

    pub fn set_agent_id(&self, agent_id: AgentId) {
        self.beam.set_agent_id(agent_id)
    }

    pub fn beam(&self) -> Rc<LaserBeam> {
        self.beam.clone()
    }
}
