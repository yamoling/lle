use std::rc::Rc;

use crate::{
    agent::{AgentId, Colour},
    tiles::Direction,
};

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

    /// Deprecated alias for [`Self::colour`], kept for one release.
    pub fn agent_id(&self) -> AgentId {
        self.colour()
    }

    /// The beam's colour. Every agent of this colour may block and cross the beam.
    pub fn colour(&self) -> Colour {
        self.beam.colour()
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

    pub fn set_colour(&self, colour: Colour) {
        self.beam.set_colour(colour)
    }

    /// Deprecated alias for [`Self::set_colour`], kept for one release.
    pub fn set_agent_id(&self, agent_id: AgentId) {
        self.set_colour(agent_id)
    }

    pub fn beam(&self) -> Rc<LaserBeam> {
        self.beam.clone()
    }
}
