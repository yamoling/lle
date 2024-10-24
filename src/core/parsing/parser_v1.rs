use crate::{AgentId, Position, Tile};

use super::{laser_config::LaserConfig, world_config::Config, ParseError};

#[derive(Default)]
struct ParsingData {
    map_string: String,
    gem_positions: Vec<Position>,
    start_positions: Vec<Vec<Position>>,
    void_positions: Vec<Position>,
    exit_positions: Vec<Position>,
    walls_positions: Vec<Position>,
    laser_configs: Vec<(Position, LaserConfig)>,
}

impl ParsingData {
    fn init(map_str: &str) -> Self {
        Self {
            map_string: map_str.into(),
            ..Default::default()
        }
    }

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
}

pub fn parse(world_str: &str) -> Result<Config, ParseError> {
    let mut data = ParsingData::init(world_str);
    let mut width = None;
    let mut row = 0usize;
    for line in world_str.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let tokens = line.split_whitespace();
        let row_width = tokens.clone().count();
        check_width(row, &mut width, row_width)?;
        for (col, token) in tokens.enumerate() {
            let pos = (row, col);
            if let Ok(tile) = Tile::try_from_str(token, row, col) {
                match tile {
                    Tile::Floor { .. } => {}
                    Tile::Wall => data.add_wall(pos),
                    Tile::Gem(..) => data.add_gem(pos),
                    Tile::Start(s) => data.add_start_position(s.start_agent_id(), pos)?,
                    Tile::Exit { .. } => data.add_exit(pos),
                    Tile::Void(..) => data.add_void(pos),
                    Tile::LaserSource(..) => {
                        let source_config = LaserConfig::from_str(token, data.n_lasers())?;
                        data.add_laser_source(pos, source_config);
                    }
                    Tile::Laser(..) => {
                        unreachable!("Lasers and LaserSources should not be parsed at this stage (i.e. without global context)")
                    }
                }
            }
        }
        row += 1;
    }
    let width = width.ok_or(ParseError::EmptyWorld)?;
    Ok(Config::new(
        width,
        row,
        world_str.into(),
        data.gem_positions,
        data.start_positions,
        data.void_positions,
        data.exit_positions,
        data.walls_positions,
        data.laser_configs,
    ))
}

/// All rows should have the same width.
pub fn check_width(
    row_num: usize,
    first_row_width: &mut Option<usize>,
    row_width: usize,
) -> Result<(), ParseError> {
    if let Some(w) = first_row_width {
        if *w != row_width {
            return Err(ParseError::InconsistentDimensions {
                expected_n_cols: *w,
                actual_n_cols: row_width,
                row: row_num,
            });
        }
    } else {
        *first_row_width = Some(row_width);
    }
    Ok(())
}
