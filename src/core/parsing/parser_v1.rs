use std::collections::BTreeMap;

use crate::{Grid, Position, agent::Colour};

use super::{
    ParseError, button_config::ButtonConfig, laser_config::LaserConfig, lift_config::LiftConfig,
    world_config::WorldConfig,
};

#[derive(Default)]
pub struct ParsingData {
    pub width: Option<usize>,
    pub height: usize,
    pub layers: usize,
    pub gem_positions: Vec<Position>,
    /// Start positions grouped by colour, in reading order. `k` occurrences of `S<c>` declare `k`
    /// agents of colour `c`. Ordered by colour so that flattening yields colour-major agent ids
    /// (see `.agents/plans/agent-colour-id.md` §3.4a): with unique tokens this reproduces the
    /// historical "agent id = token number" assignment exactly.
    pub start_positions: BTreeMap<Colour, Vec<Position>>,
    pub void_positions: Vec<Position>,
    pub exit_positions: Vec<Position>,
    pub walls_positions: Vec<Position>,
    pub box_positions: Vec<Position>,
    pub laser_configs: Vec<(Position, LaserConfig)>,
    pub lift_configs: Vec<(Position, LiftConfig)>,
    pub button_configs: Vec<(Position, ButtonConfig)>,
}

impl ParsingData {
    pub fn add_wall(&mut self, pos: Position) {
        self.walls_positions.push(pos);
    }

    pub fn add_laser_source(&mut self, pos: Position, config: LaserConfig) {
        self.laser_configs.push((pos, config));
        self.walls_positions.push(pos);
    }

    pub fn add_lift(&mut self, pos: Position, config: LiftConfig) {
        self.lift_configs.push((pos, config));
    }

    pub fn add_button(&mut self, pos: Position, config: ButtonConfig) {
        self.button_configs.push((pos, config));
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

    pub fn add_box(&mut self, pos: Position) {
        self.box_positions.push(pos);
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

    fn increase_height(&mut self) {
        self.height += 1;
    }

    pub fn add_row(&mut self, n_cols: usize, line: &str, row: usize) -> Result<(), ParseError> {
        if let Some(w) = self.width {
            if w != n_cols {
                return Err(ParseError::Inconsistent2Dimensions {
                    row_str: line.to_string(),
                    expected_n_cols: w,
                    actual_n_cols: n_cols,
                    row,
                });
            }
        } else {
            self.width = Some(n_cols);
        }
        Ok(())
    }
    pub fn add_layer(&mut self, hw: (usize, usize)) -> Result<(), ParseError> {
        // TODO refactor
        match (self.height, self.width) {
            (h, Some(w)) => {
                if hw != (h, w) {
                    return Err(ParseError::Inconsistent3Dimensions {
                        expected_n_dims: (h, w),
                        actual_n_dims: hw,
                        layer: self.layers, // dont realy care about the layer number in the error message
                    });
                }
            }
            _ => {
                return Err(ParseError::EmptyWorld);
            }
        }
        self.layers += 1;
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
        let layers = self.layers; //? need to be consistent with the default value of layers in ParsingData
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
            layers,
            self.gem_positions,
            starts,
            self.void_positions,
            self.exit_positions,
            self.walls_positions,
            self.box_positions,
            self.laser_configs,
            self.lift_configs,
            self.button_configs,
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
    if config
        .boxes()
        .iter()
        .any(|b| config.gems().contains(b) || config.exits().contains(b))
    {
        return Err(());
    }
    let mut res =
        Grid::<String>::new(config.width(), config.height(), config.layers()).default_init();
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
        res.replace_at(&pos, format!("S{colour}"));
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
    for pos in config.boxes() {
        res.replace_at(&pos, "#".into());
    }
    for (pos, config) in config.sources() {
        res.replace_at(&pos, config.to_string());
    }
    for (pos, config) in config.lifts() {
        res.replace_at(&pos, config.to_string());
    }
    for (pos, config) in config.buttons() {
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
            data.add_layer((row, n_cols))?;
            row = 0;
            layer += 1;
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
                '#' => data.add_box(pos),
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
                'T' => {
                    let lift_config = LiftConfig::from_str(token)?;
                    data.add_lift(pos, lift_config);
                }
                'B' => {
                    let button_config = ButtonConfig::from_str(token)?;
                    data.add_button(pos, button_config);
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
        data.add_row(n_cols, line, row)?;
        if layer == 0 {
            data.increase_height();
        }
        row += 1;
    }
    data.add_layer((row, n_cols))?;
    data.try_into()
}

#[cfg(test)]
#[path = "../../unit_tests/test_parser_v1.rs"]
mod tests;
