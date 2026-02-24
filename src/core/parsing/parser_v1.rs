use crate::{AgentId, Grid, Position};

use super::{ParseError, laser_config::LaserConfig, world_config::WorldConfig};

#[derive(Default)]
pub struct ParsingData {
    pub width: Option<usize>,
    pub height: usize,
    pub layers: Option<usize>,
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
            //? why one start position if we create a vector of positions per agent?
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
                return Err(ParseError::Inconsistent2Dimensions {
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
    pub fn add_layer(&mut self, wh: (usize, usize)) -> Result<(), ParseError> {
        // TODO refactor
        match (self.width, self.height) {
            (Some(w), h) => {
                if wh != (w, h) {
                    return Err(ParseError::Inconsistent3Dimensions {
                        expected_n_dims: (w, h),
                        actual_n_dims: wh,
                        layer: self.layers.unwrap_or(1), // dont realy care about the layer number in the error message
                    });
                }
            }
            (None, h) => {
                if wh.1 != h {
                    return Err(ParseError::Inconsistent3Dimensions {
                        expected_n_dims: (wh.0, h),
                        actual_n_dims: wh,
                        layer: self.layers.unwrap_or(1),
                    });
                }
                self.width = Some(wh.0);
            }
        }
        self.layers = Some(self.layers.unwrap_or(1) + 1);
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
        let layers = self.layers.unwrap_or(1); //? need to be consistent with the default value of layers in ParsingData
        Ok(WorldConfig::new(
            width,
            self.height,
            layers,
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
    let mut res =
        Grid::<String>::new(config.width(), config.height(), config.layers()).default_init();
    for (agent_num, pos) in config.random_starts().iter().enumerate() {
        if pos.len() > 1 {
            return Err(());
        }
        let pos = pos[0];
        res.replace_at(&pos, format!("S{agent_num}"));
    }

    for pos in config.gems() {
        res.replace_at(&pos, "G".into());
    }
    for pos in config.walls() {
        res.replace_at(&pos, "@".into());
    }
    for pos in config.exits() {
        res.replace_at(&pos, "X".into());
    }
    for pos in config.voids() {
        res.replace_at(&pos, "V".into());
    }
    for (pos, config) in config.sources() {
        res.replace_at(&pos, config.to_string());
    }
    Ok((&res).into())
}

pub fn parse(world_str: &str) -> Result<WorldConfig, ParseError> {
    let mut data = ParsingData::default();

    let mut layer = 0usize; // there must be at least one layer but the index of the first layer is 0
    let mut row = 0usize;
    let mut n_cols = 0usize;
    for line in world_str.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if line.starts_with(';') {
            layer += 1;
            assert!(n_cols > 0 && row > 0); // there should be at least one row in the previous layer
            data.add_layer((n_cols, row))?;
            row = 0;
            continue;
        }
        let tokens = line.split_whitespace();
        n_cols = 0usize;
        for (col, token) in tokens.enumerate() {
            n_cols += 1;
            let pos = Position {
                i: row,
                j: col,
                k: layer,
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
        row += 1;
    }
    data.try_into()
}

#[cfg(test)]
mod tests {
    use crate::ParseError;

    use super::parse;

    #[test]
    fn test_laser_kill_on_spawn() {
        match parse(
            "
            L1S  X  .
            S0 S1  X
            ",
        ) {
            Ok(config) => {
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
            Err(e) => panic!("Unexpected error during parsing: {:?}", e),
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
