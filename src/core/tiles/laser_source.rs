use std::rc::Rc;

use crate::{agent::AgentId, tiles::Direction, ParseError, Position};

use super::LaserBeam;

pub type LaserId = usize;

#[derive(Clone, Debug)]
pub struct LaserSource {
    beam: Rc<LaserBeam>,
}

impl LaserSource {
    /// Note there is no "TryFrom" implementation for LaserSource because we need the laser_id.
    pub fn from_str(value: &str, laser_id: LaserId) -> Result<LaserBuilder, ParseError> {
        let direction = Direction::try_from(value.chars().last().unwrap()).unwrap();
        let agent_id = match (&value[1..2]).parse::<AgentId>() {
            Ok(agent_id) => agent_id,
            Err(_) => {
                return Err(ParseError::InvalidAgentId {
                    given_agent_id: value[1..2].to_string(),
                })
            }
        };
        Ok(LaserBuilder::new(direction, agent_id, laser_id))
    }

    pub fn is_enabled(&self) -> bool {
        self.beam.is_enabled()
    }

    pub fn agent_id(&self) -> AgentId {
        self.beam.agent_id()
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

pub struct LaserBuilder {
    pub direction: Direction,
    pub agent_id: AgentId,
    pub laser_id: LaserId,
    pub beam_pos: Vec<Position>,
}

impl LaserBuilder {
    fn new(direction: Direction, agent_id: AgentId, laser_id: LaserId) -> Self {
        Self {
            direction,
            agent_id,
            laser_id,
            beam_pos: vec![],
        }
    }

    pub fn extend_beam(&mut self, pos: Position) {
        self.beam_pos.push(pos);
    }

    pub fn build(&self) -> (LaserSource, Vec<Position>) {
        let beam = Rc::new(LaserBeam::new(
            self.beam_pos.len(),
            self.agent_id,
            self.direction,
            self.laser_id,
        ));
        (LaserSource { beam }, self.beam_pos.clone())
    }
}
