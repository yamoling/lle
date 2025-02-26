use std::{cmp::Ordering, collections::HashSet};

use itertools::Itertools;
use serde::{Deserialize, Serialize};

use crate::{ParseError, Position};

use super::PositionsConfig;

#[derive(Deserialize, Serialize, Debug, Default)]
#[serde(deny_unknown_fields)]
pub struct AgentConfig {
    #[serde(default, alias = "start_positions")]
    pub starts: Vec<PositionsConfig>,
}

impl AgentConfig {
    pub fn compute_start_positions(
        &self,
        global_start_positions: &[Position],
        world_width: usize,
        world_height: usize,
        wall_positions: &[Position],
        exit_positions: &[Position],
    ) -> Result<Vec<Position>, ParseError> {
        let mut res: HashSet<Position> =
            HashSet::from_iter(global_start_positions.to_vec().into_iter());
        for start in &self.starts {
            for pos in start.to_positions(world_width, world_height)? {
                res.insert(pos);
            }
        }
        for wall in wall_positions {
            res.remove(wall);
        }
        for exit in exit_positions {
            res.remove(exit);
        }
        Ok(res
            .into_iter()
            .sorted_by(|pos1, pos2| {
                // Sort the items by how close they are to the top left corner, priorizing the row
                // over the column.
                let pos1_score = pos1.i * world_width + pos1.j;
                let pos2_score = pos2.i * world_width + pos2.j;
                if pos1_score < pos2_score {
                    return Ordering::Less;
                }
                if pos1_score > pos2_score {
                    return Ordering::Greater;
                }
                if pos1.i < pos2.i {
                    return Ordering::Less;
                }
                if pos1.i > pos2.j {
                    return Ordering::Greater;
                }
                Ordering::Equal
            })
            .collect())
    }
}
