use std::rc::Rc;

use crate::{
    tiles::{Direction, LaserBeam, LaserId, LaserSource},
    AgentId,
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
                })
            }
        };
        Ok(Self {
            direction,
            agent_id,
            laser_id,
        })
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

#[cfg(test)]
mod test {
    use crate::core::parsing::laser_config::LaserConfig;

    #[test]
    fn laser_source_from_str() {
        let source = LaserConfig::from_str("L0E", 0).unwrap().build(3);
        assert_eq!(source.direction(), crate::tiles::Direction::East);
        assert_eq!(source.agent_id(), 0);
        assert_eq!(source.laser_id(), 0);

        let source = LaserConfig::from_str("L1W", 25).unwrap().build(5);
        assert_eq!(source.direction(), crate::tiles::Direction::West);
        assert_eq!(source.agent_id(), 1);
        assert_eq!(source.laser_id(), 25);

        let source = LaserConfig::from_str("L2N", 0).unwrap().build(10);
        assert_eq!(source.direction(), crate::tiles::Direction::North);
        assert_eq!(source.agent_id(), 2);
        assert_eq!(source.laser_id(), 0);

        let source = LaserConfig::from_str("L3S", 0).unwrap().build(800);
        assert_eq!(source.direction(), crate::tiles::Direction::South);
        assert_eq!(source.agent_id(), 3);
        assert_eq!(source.laser_id(), 0);
    }
}
