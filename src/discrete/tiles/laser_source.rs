use crate::shared::map::LaserConfig;
use std::rc::Rc;

use crate::{AgentId, shared::Colour, tiles::CardinalDirection};

use super::LaserBeam;

pub use crate::shared::LaserId;

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

    pub fn direction(&self) -> CardinalDirection {
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

impl LaserSource {
    /// The source described by `config`, whose beam covers `beam_length` cells.
    pub fn from_config(config: &LaserConfig, beam_length: usize) -> Self {
        let beam = Rc::new(LaserBeam::new(
            beam_length,
            config.colour,
            config.direction,
            config.laser_id,
        ));
        LaserSource::new(beam)
    }
}

impl From<&LaserSource> for LaserConfig {
    fn from(source: &LaserSource) -> Self {
        Self {
            direction: source.direction(),
            colour: source.colour(),
            laser_id: source.laser_id(),
        }
    }
}

impl From<LaserSource> for LaserConfig {
    fn from(source: LaserSource) -> Self {
        Self::from(&source)
    }
}
