use std::{collections::HashSet, vec};

use crate::{
    tiles::{Gem, Laser, Tile, Void},
    Position, World,
};

use crate::ParseError;

use super::laser_config::LaserConfig;

pub struct Config {
    width: usize,
    height: usize,
    map_string: String,
    gem_positions: Vec<Position>,
    start_positions: Vec<Vec<Position>>,
    void_positions: Vec<Position>,
    exit_positions: Vec<Position>,
    walls_positions: Vec<Position>,
    source_configs: Vec<(Position, LaserConfig)>,
}

impl Config {
    pub fn new(
        width: usize,
        height: usize,
        map_string: String,
        gem_positions: Vec<Position>,
        start_positions: Vec<Vec<Position>>,
        void_positions: Vec<Position>,
        exit_positions: Vec<Position>,
        walls_positions: Vec<Position>,
        source_configs: Vec<(Position, LaserConfig)>,
    ) -> Self {
        Self {
            width,
            height,
            map_string,
            gem_positions,
            start_positions,
            void_positions,
            exit_positions,
            walls_positions,
            source_configs,
        }
    }

    pub fn to_world(self) -> Result<World, ParseError> {
        self.validate()?;
        let (grid, lasers_positions) = self.make_grid();
        let source_positions = self.source_configs.iter().map(|(pos, _)| *pos).collect();
        let start_positions = self.start_positions.into_iter().flatten().collect();
        Ok(World::new(
            grid,
            self.gem_positions,
            start_positions,
            self.void_positions,
            self.exit_positions,
            self.walls_positions,
            source_positions,
            lasers_positions,
            &self.map_string,
        ))
    }

    fn validate(&self) -> Result<(), ParseError> {
        // There are some agents
        if self.start_positions.is_empty() {
            return Err(ParseError::NoAgents);
        }
        // There are enough start/exit tiles
        if self.exit_positions.len() < self.n_agents() {
            return Err(ParseError::NotEnoughExitTiles {
                n_starts: self.n_agents(),
                n_exits: self.exit_positions.len(),
            });
        }
        // All agents have at least one start tile
        for (agent_id, starts) in self.start_positions.iter().enumerate() {
            if starts.is_empty() {
                return Err(ParseError::AgentWithoutStart { agent_id });
            }
        }

        // Check that there are enough start tiles for all agents
        let n_starts = self
            .start_positions
            .iter()
            .fold(0usize, |sum, current| sum + current.len());
        if n_starts < self.n_agents() {
            return Err(ParseError::NotEnoughStartTiles {
                n_starts,
                n_agents: self.n_agents(),
            });
        }

        // Check that there are no lasers with an agent ID that does not exist
        for (_, source) in self.source_configs.iter() {
            if source.agent_id >= self.n_agents() {
                return Err(ParseError::InvalidLaserSourceAgentId {
                    asked_id: source.agent_id,
                    n_agents: self.n_agents(),
                });
            }
        }

        Ok(())
    }

    pub fn n_agents(&self) -> usize {
        self.start_positions.len()
    }

    fn make_grid(&self) -> (Vec<Vec<Tile>>, Vec<Position>) {
        let mut grid = Vec::with_capacity(self.height);
        for _ in 0..self.height {
            let mut row = Vec::with_capacity(self.width);
            for _ in 0..self.width {
                row.push(Tile::Floor { agent: None });
            }
            grid.push(row);
        }
        for pos in &self.gem_positions {
            grid[pos.0][pos.1] = Tile::Gem(Gem::default());
        }
        for pos in &self.exit_positions {
            grid[pos.0][pos.1] = Tile::Exit { agent: None };
        }
        for pos in &self.void_positions {
            grid[pos.0][pos.1] = Tile::Void(Void::default());
        }
        for pos in &self.walls_positions {
            grid[pos.0][pos.1] = Tile::Wall;
        }
        let laser_positions = laser_setup(&mut grid, &self.source_configs)
            .into_iter()
            .collect();
        (grid, laser_positions)
    }
}

pub fn parse(world_str: &str) -> Result<World, ParseError> {
    // let config = Config::parse(world_str)?;
    // Ok(config.to_world())
    todo!()
}

/// Place the laser sources and wrap the required tiles behind a
/// `Laser` tile.
fn laser_setup(
    grid: &mut Vec<Vec<Tile>>,
    laser_configs: &[(Position, LaserConfig)],
) -> HashSet<Position> {
    let mut laser_positions = HashSet::new();
    let width = grid[0].len() as i32;
    let height: i32 = grid.len() as i32;
    for (pos, source) in laser_configs {
        let mut beam_positions = vec![];
        let delta = source.direction.delta();
        let (mut i, mut j) = (pos.0 as i32, pos.1 as i32);
        (i, j) = ((i + delta.0), (j + delta.1));
        while i >= 0 && j >= 0 && i < height && j < width {
            let pos = (i as usize, j as usize);
            if !grid[pos.0][pos.1].is_waklable() {
                break;
            }
            beam_positions.push(pos);
            (i, j) = ((i + delta.0), (j + delta.1));
        }
        laser_positions.extend(&beam_positions);
        let source = source.build(beam_positions.len());
        for (i, pos) in beam_positions.into_iter().enumerate() {
            let wrapped = grid[pos.0].remove(pos.1);
            let laser = Tile::Laser(Laser::new(wrapped, source.beam(), i));
            grid[pos.0].insert(pos.1, laser);
        }
        grid[pos.0][pos.1] = Tile::LaserSource(source);
    }
    laser_positions
}