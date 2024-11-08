use serde::{Deserialize, Serialize};

use crate::{ParseError, Position};

use super::PositionsConfig;

#[derive(Deserialize, Serialize, Debug, Default)]
#[serde(deny_unknown_fields)]
pub struct AgentConfig {
    #[serde(default)]
    pub start_positions: Vec<PositionsConfig>,
}

impl AgentConfig {
    pub fn compute_start_positions(
        &self,
        world_width: usize,
        world_height: usize,
        wall_positions: &[Position],
    ) -> Result<Vec<Position>, ParseError> {
        let mut res = vec![];
        for start in &self.start_positions {
            let mut start = start
                .to_positions(world_width, world_height)?
                .into_iter()
                .filter(|pos| !wall_positions.contains(pos))
                .collect();
            res.append(&mut start);
        }
        Ok(res)
    }
}
