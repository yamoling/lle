use std::collections::BTreeMap;

use crate::{Position, agent::Colour};

use super::{ParseError, laser_config::LaserConfig, world_config::WorldConfig};

#[derive(Default)]
pub struct ParsingData {
    pub width: Option<usize>,
    pub height: usize,
    pub gem_positions: Vec<Position>,
    /// Start positions grouped by colour, in reading order. `k` occurrences of `S<c>` declare `k`
    /// agents of colour `c`. Ordered by colour so that flattening yields colour-major agent ids
    /// (see `.agents/plans/agent-colour-id.md` §3.4a): with unique tokens this reproduces the
    /// historical "agent id = token number" assignment exactly.
    pub start_positions: BTreeMap<Colour, Vec<Position>>,
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

    /// Declare one agent of colour `colour` starting at `pos`. Repeating a token declares
    /// several agents of that colour.
    pub fn add_start_position(&mut self, colour: Colour, pos: Position) {
        self.start_positions.entry(colour).or_default().push(pos);
    }

    /// The colour of each agent, in colour-major agent-id order.
    pub fn agent_colours(&self) -> Vec<Colour> {
        self.start_positions
            .iter()
            .flat_map(|(&colour, starts)| std::iter::repeat_n(colour, starts.len()))
            .collect()
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

    pub fn add_row(&mut self, n_cols: usize, line: &str) -> Result<(), ParseError> {
        if let Some(w) = self.width {
            if w != n_cols {
                return Err(ParseError::InconsistentDimensions {
                    row_str: line.to_string(),
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
        let colours = self.agent_colours();
        // One agent per start tile, ordered by (colour, reading order).
        let starts = self
            .start_positions
            .into_values()
            .flatten()
            .map(|pos| vec![pos])
            .collect();
        Ok(WorldConfig::new(
            width,
            self.height,
            self.gem_positions,
            starts,
            self.void_positions,
            self.exit_positions,
            self.walls_positions,
            self.laser_configs,
            colours,
        ))
    }
}

/// Render a config as a v1 ASCII world string, or `Err(())` when v1 cannot express it.
///
/// v1 cannot express several possible start positions for one agent, and it re-derives agent ids
/// by `(colour, reading order)` on reparse — so a world whose same-colour agents are not already
/// in reading order would come back with those agents swapped. Both cases return `Err(())`, and
/// `WorldConfig::Display` falls back to TOML (see `.agents/plans/agent-colour-id.md` §3.4d).
pub fn to_v1_string(config: &WorldConfig) -> Result<String, ()> {
    let mut res = vec![vec![String::from(" . "); config.width()]; config.height()];
    let mut previous_of_colour: std::collections::HashMap<usize, Position> =
        std::collections::HashMap::new();
    for (agent_num, pos) in config.random_starts().iter().enumerate() {
        if pos.len() > 1 {
            return Err(());
        }
        let pos = pos[0];
        let colour = *config.colours().get(agent_num).ok_or(())?;
        // Agents of one colour must already be in reading order, or the emission is lossy.
        if let Some(previous) = previous_of_colour.insert(colour, pos)
            && (previous.i, previous.j) > (pos.i, pos.j)
        {
            return Err(());
        }
        res[pos.i][pos.j] = format!("S{colour} ");
    }

    for pos in config.gems() {
        res[pos.i][pos.j] = " G ".into();
    }
    for pos in config.walls() {
        res[pos.i][pos.j] = " @ ".into();
    }
    for pos in config.exits() {
        res[pos.i][pos.j] = " X ".into();
    }
    for pos in config.voids() {
        res[pos.i][pos.j] = " V ".into();
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
                    let colour = token[1..].parse().map_err(|_| ParseError::InvalidAgentId {
                        given_agent_id: token[1..].into(),
                    })?;
                    data.add_start_position(colour, pos);
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
        data.add_row(n_cols, line)?;
    }
    data.try_into()
}

#[cfg(test)]
#[path = "../../unit_tests/test_parser_v1.rs"]
mod tests;
