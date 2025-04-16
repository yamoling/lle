use crate::{AgentId, Position};

use super::{ParseError, laser_config::LaserConfig, world_config::WorldConfig};

#[derive(Default)]
pub struct ParsingData {
    pub width: Option<usize>,
    pub height: usize,
    pub gem_positions: Vec<Position>,
    pub start_positions: Vec<Vec<Position>>,
    pub void_positions: Vec<Position>,
    pub exit_positions: Vec<Position>,
    pub walls_positions: Vec<Position>,
    pub laser_configs: Vec<(Position, LaserConfig)>,
}

impl ParsingData {
    pub fn add_wall(&mut self, pos: Position) {
        self.walls_positions.push(pos);
    }

    pub fn add_laser_source(&mut self, pos: Position, config: LaserConfig) {
        self.laser_configs.push((pos, config));
        self.walls_positions.push(pos);
    }

    pub fn add_start_position(
        &mut self,
        agent_id: AgentId,
        pos: Position,
    ) -> Result<(), ParseError> {
        while self.start_positions.len() <= agent_id as usize {
            self.start_positions.push(Vec::new());
        }
        if !self.start_positions[agent_id].is_empty() {
            return Err(ParseError::DuplicateStartTile {
                agent_id,
                start1: self.start_positions[agent_id][0],
                start2: pos,
            });
        }
        self.start_positions[agent_id].push(pos);
        Ok(())
    }

    pub fn add_gem(&mut self, pos: Position) {
        self.gem_positions.push(pos);
    }

    pub fn add_void(&mut self, pos: Position) {
        self.void_positions.push(pos);
    }

    pub fn add_exit(&mut self, pos: Position) {
        self.exit_positions.push(pos);
    }

    fn n_lasers(&self) -> usize {
        self.laser_configs.len()
    }

    pub fn add_row(&mut self, n_cols: usize) -> Result<(), ParseError> {
        if let Some(w) = self.width {
            if w != n_cols {
                return Err(ParseError::InconsistentDimensions {
                    expected_n_cols: w,
                    actual_n_cols: n_cols,
                    row: self.height,
                });
            }
        } else {
            self.width = Some(n_cols);
        }
        self.height += 1;
        Ok(())
    }
}

impl TryInto<WorldConfig> for ParsingData {
    type Error = ParseError;
    fn try_into(self) -> Result<WorldConfig, Self::Error> {
        if self.height == 0 {
            return Err(ParseError::EmptyWorld);
        }
        let width = self.width.ok_or(ParseError::MissingWidth)?;
        Ok(WorldConfig::new(
            width,
            self.height,
            self.gem_positions,
            self.start_positions,
            self.void_positions,
            self.exit_positions,
            self.walls_positions,
            self.laser_configs,
        ))
    }
}

pub fn to_v1_string(config: &WorldConfig) -> Result<String, ()> {
    let mut res = vec![vec![String::from("."); config.width()]; config.height()];
    for (agent_num, pos) in config.random_starts().iter().enumerate() {
        if pos.len() > 1 {
            return Err(());
        }
        let pos = pos[0];
        res[pos.i][pos.j] = format!("S{agent_num}");
    }

    for pos in config.gems() {
        res[pos.i][pos.j] = "G".into();
    }
    for pos in config.walls() {
        res[pos.i][pos.j] = "@".into();
    }
    for pos in config.exits() {
        res[pos.i][pos.j] = "X".into();
    }
    for pos in config.voids() {
        res[pos.i][pos.j] = "V".into();
    }
    for (pos, config) in config.sources() {
        res[pos.i][pos.j] = config.to_string();
    }
    Ok(res
        .into_iter()
        .map(|row| row.join(" "))
        .collect::<Vec<String>>()
        .join("\n"))
}

pub fn parse(world_str: &str) -> Result<WorldConfig, ParseError> {
    let mut data = ParsingData::default();
    for line in world_str.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let tokens = line.split_whitespace();
        let mut n_cols = 0usize;
        for (col, token) in tokens.enumerate() {
            n_cols += 1;
            let pos = Position {
                i: data.height,
                j: col,
            };
            match token.to_uppercase().chars().next().unwrap() {
                '.' => {}
                'G' => data.add_gem(pos),
                '@' => data.add_wall(pos),
                'X' => data.add_exit(pos),
                'V' => data.add_void(pos),
                'S' => {
                    let agent_id = token[1..].parse().map_err(|_| ParseError::InvalidAgentId {
                        given_agent_id: token[1..].into(),
                    })?;
                    data.add_start_position(agent_id, pos)?;
                }
                'L' => {
                    let source_config = LaserConfig::from_str(token, data.n_lasers())?;
                    data.add_laser_source(pos, source_config);
                }
                _ => {
                    return Err(ParseError::InvalidTile {
                        tile_str: token.into(),
                        line: pos.i,
                        col: pos.j,
                    });
                }
            }
        }
        data.add_row(n_cols)?;
    }
    data.try_into()
}

#[cfg(test)]
mod tests {
    use crate::ParseError;

    use super::parse;

    #[test]
    fn test_laser_kill_on_spawn() {
        let config = parse(
            "
        L1S  X  .
         S0 S1  X
        ",
        )
        .unwrap();
        let world = config.to_world();
        match world {
            Ok(_) => panic!(
                "The start location of agent 0 should have been removed and no remaining start position remains for agent 0"
            ),
            Err(ParseError::AgentWithoutStart { .. }) => {}
            Err(ParseError::NotEnoughExitTiles { .. }) => {}
            Err(e) => panic!("Unexpected error: {:?}", e),
        }
    }

    #[test]
    fn test_laser_blocked_on_spawn() {
        let config = parse(
            "
        L1E . S1 S0 X
        L0E .  .  . X
        ",
        )
        .unwrap();
        let world = config.to_world();
        match world {
            Ok(_) => {}
            Err(ParseError::AgentWithoutStart { .. }) => panic!(
                "The start location of agent 0 should have been removed and no remaining start position remains for agent 0"
            ),
            Err(ParseError::NotEnoughExitTiles { .. }) => panic!("There are enough exit tiles"),
            Err(e) => panic!("Unexpected error: {:?}", e),
        }
    }
}
