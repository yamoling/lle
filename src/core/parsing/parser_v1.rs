use super::{
    ParseError, button_config::ButtonConfig, laser_config::LaserConfig, lift_config::LiftConfig,
    world_config::WorldConfig,
};
use crate::{AgentId, Grid, Position, log_warn};
use crate::{log_debug, log_info};

#[derive(Default)]
pub struct ParsingData {
    pub width: Option<usize>,
    pub height: usize,
    pub layers: usize,
    pub gem_positions: Vec<Position>,
    pub start_positions: Vec<Vec<Position>>,
    pub void_positions: Vec<Position>,
    pub exit_positions: Vec<Position>,
    pub walls_positions: Vec<Position>,
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

    fn increase_height(&mut self) {
        self.height += 1;
    }

    pub fn add_row(&mut self, n_cols: usize, line: &str, row: usize) -> Result<(), ParseError> {
        log_debug!("Adding row with {} columns: {}", n_cols, line);
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
        log_debug!("Adding layer with dimensions: {}x{}", hw.0, hw.1);
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
                log_warn!("Attempted to add a layer with no rows parsed (empty world)");
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
        log_info!("begin converting ParsingData to WorldConfig");
        if self.height == 0 {
            return Err(ParseError::EmptyWorld);
        }
        let width = self.width.ok_or(ParseError::MissingWidth)?;
        let layers = self.layers; //? need to be consistent with the default value of layers in ParsingData
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
            self.lift_configs,
            self.button_configs,
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
            log_debug!(
                "Finished parsing layer {layer} with dimensions: {}x{}",
                row,
                n_cols
            );
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
    log_debug!(
        "Finished parsing world with dimensions: {}x{}x{}",
        data.height,
        data.width.unwrap(),
        data.layers // will be done downstream
    );
    data.try_into()
}

#[cfg(test)]
mod tests {
    use crate::{
        ParseError, Position,
        tiles::{Direction, Tile},
    };

    use super::parse;

    #[test]
    fn test_parse_lift_and_button() {
        let config = parse(
            "
            S0 .  TN0A1
            .  B0 .
            .  .  X
            ",
        )
        .unwrap();
        let world = config.to_world().unwrap();

        match world.at(&Position { i: 0, j: 2, k: 0 }) {
            Some(Tile::Lift(lift)) => {
                assert_eq!(lift.direction(), Direction::North);
                assert_eq!(lift.group_id(), 0);
                assert_eq!(lift.authorized_agent_id(), Some(1));
            }
            other => panic!("Expected a Lift tile, got {:?}", other),
        }

        match world.at(&Position { i: 1, j: 1, k: 0 }) {
            Some(Tile::Button(button)) => {
                assert_eq!(button.group_id(), 0);
                assert_eq!(button.authorized_agent_id(), None);
            }
            other => panic!("Expected a Button tile, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_lift_button_invalid_group_id() {
        match parse(
            "
            S0 TNx
            .  X
            ",
        ) {
            Err(ParseError::InvalidGroupId { .. }) => {}
            other => panic!("Expected ParseError::InvalidGroupId, got {:?}", other),
        }
    }

    #[test]
    fn test_lift_button_round_trip() {
        let config = parse(
            "
            S0 .  TN0A1
            .  B0 .
            .  .  X
            ",
        )
        .unwrap();
        let as_string = super::to_v1_string(&config).unwrap();
        let reparsed = parse(&as_string).unwrap();
        let world = reparsed.to_world().unwrap();

        match world.at(&Position { i: 0, j: 2, k: 0 }) {
            Some(Tile::Lift(lift)) => {
                assert_eq!(lift.direction(), Direction::North);
                assert_eq!(lift.group_id(), 0);
                assert_eq!(lift.authorized_agent_id(), Some(1));
            }
            other => panic!("Expected a Lift tile, got {:?}", other),
        }
        match world.at(&Position { i: 1, j: 1, k: 0 }) {
            Some(Tile::Button(button)) => {
                assert_eq!(button.group_id(), 0);
            }
            other => panic!("Expected a Button tile, got {:?}", other),
        }
    }

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

    #[test]
    fn test_empty_string_returns_empty_world_not_panic() {
        match parse("") {
            Err(ParseError::EmptyWorld) => {}
            other => panic!("Expected ParseError::EmptyWorld, got {:?}", other),
        }
    }

    #[test]
    fn test_leading_semicolon_returns_parse_error() {
        match parse(";\nS0 X") {
            Err(ParseError::EmptyWorld) => {}
            other => panic!("Expected ParseError::EmptyWorld, got {:?}", other),
        }
    }

    #[test]
    fn test_doubled_semicolon_returns_parse_error() {
        match parse("S0 X\n;\n;") {
            Err(ParseError::Inconsistent3Dimensions { .. }) => {}
            other => panic!("Expected ParseError::Inconsistent3Dimensions, got {:?}", other),
        }
    }

    #[test]
    fn test_row_length_mismatch_returns_inconsistent_2d() {
        match parse("X S0 .\n. .") {
            Err(ParseError::Inconsistent2Dimensions {
                expected_n_cols,
                actual_n_cols,
                row,
                ..
            }) => {
                assert_eq!(expected_n_cols, 3);
                assert_eq!(actual_n_cols, 2);
                assert_eq!(row, 1);
            }
            other => panic!("Expected ParseError::Inconsistent2Dimensions, got {:?}", other),
        }
    }
}
