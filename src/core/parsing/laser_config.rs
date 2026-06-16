use std::rc::Rc;

use crate::{
    AgentId,
    tiles::{Direction, LaserBeam, LaserId, LaserSource},
};

use super::ParseError;

#[derive(Debug)]
pub struct LaserConfig {
    pub direction: Direction,
    pub agent_id: AgentId,
    pub laser_id: LaserId,
}

impl LaserConfig {
    /// Note there is no "TryFrom" implementation for LaserSource because we need the laser_id.
    pub fn from_str(value: &str, laser_id: LaserId) -> Result<LaserConfig, ParseError> {
        let direction = Direction::try_from(value.chars().last().unwrap()).unwrap();
        let agent_id = match (&value[1..2]).parse::<AgentId>() {
            Ok(agent_id) => agent_id,
            Err(_) => {
                return Err(ParseError::InvalidAgentId {
                    given_agent_id: value[1..2].to_string(),
                });
            }
        };
        Ok(Self {
            direction,
            agent_id,
            laser_id,
        })
    }

    pub fn to_string(&self) -> String {
        format!("L{}{}", self.agent_id, self.direction.to_file_string())
    }

    pub fn build(&self, beam_length: usize) -> LaserSource {
        let beam = Rc::new(LaserBeam::new(
            beam_length,
            self.agent_id,
            self.direction,
            self.laser_id,
        ));
        LaserSource::new(beam)
    }
}

impl From<&LaserSource> for LaserConfig {
    fn from(source: &LaserSource) -> Self {
        Self {
            direction: source.direction(),
            agent_id: source.agent_id(),
            laser_id: source.laser_id(),
        }
    }
}

impl From<LaserSource> for LaserConfig {
    fn from(source: LaserSource) -> Self {
        Self::from(&source)
    }
}

#[cfg(test)]
#[path = "../../unit_tests/test_laser_config.rs"]
mod test;
