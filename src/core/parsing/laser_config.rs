use std::rc::Rc;

use crate::{
    agent::Colour,
    tiles::{CardinalDirection, LaserBeam, LaserId, LaserSource},
};

use super::ParseError;

#[derive(Clone, Debug)]
pub struct LaserConfig {
    pub direction: CardinalDirection,
    /// The colour of the beam: agents of this colour block it, the others die in it.
    pub colour: Colour,
    pub laser_id: LaserId,
}

impl LaserConfig {
    /// Note there is no "TryFrom" implementation for LaserSource because we need the laser_id.
    pub fn from_str(value: &str, laser_id: LaserId) -> Result<LaserConfig, ParseError> {
        let direction = CardinalDirection::try_from(value.chars().last().unwrap()).unwrap();
        let colour_str = &value[1..value.len() - 1];
        let colour = match colour_str.parse::<Colour>() {
            Ok(colour) => colour,
            Err(_) => {
                return Err(ParseError::InvalidAgentId {
                    given_agent_id: colour_str.to_string(),
                });
            }
        };
        Ok(Self {
            direction,
            colour,
            laser_id,
        })
    }

    pub fn build(&self, beam_length: usize) -> LaserSource {
        let beam = Rc::new(LaserBeam::new(
            beam_length,
            self.colour,
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

impl std::fmt::Display for LaserConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "L{}{}", self.colour, self.direction.to_file_string())
    }
}

#[cfg(test)]
#[path = "../../unit_tests/test_laser_config.rs"]
mod test;
