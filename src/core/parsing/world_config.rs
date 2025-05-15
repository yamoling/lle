use std::{collections::HashSet, vec};

use crate::{
    Position, World,
    tiles::{Gem, Laser, Tile, Void},
};

use crate::ParseError;

use super::{laser_config::LaserConfig, parser_v1::to_v1_string, toml::TomlConfig};

#[derive(Debug)]
pub struct WorldConfig {
    width: usize,
    height: usize,
    gems: Vec<Position>,
    random_starts: Vec<Vec<Position>>,
    voids: Vec<Position>,
    exits: Vec<Position>,
    walls: Vec<Position>,
    lasers: Vec<(Position, LaserConfig)>,
}

impl WorldConfig {
    pub fn new(
        width: usize,
        height: usize,
        gem_positions: Vec<Position>,
        random_start_positions: Vec<Vec<Position>>,
        void_positions: Vec<Position>,
        exit_positions: Vec<Position>,
        walls_positions: Vec<Position>,
        source_configs: Vec<(Position, LaserConfig)>,
    ) -> Self {
        Self {
            width,
            height,
            gems: gem_positions,
            random_starts: random_start_positions,
            voids: void_positions,
            exits: exit_positions,
            walls: walls_positions,
            lasers: source_configs,
        }
    }

    pub fn exits(&self) -> &Vec<Position> {
        &self.exits
    }

    pub fn random_starts(&self) -> &Vec<Vec<Position>> {
        &self.random_starts
    }

    pub fn width(&self) -> usize {
        self.width
    }

    pub fn height(&self) -> usize {
        self.height
    }

    pub fn voids(&self) -> &Vec<Position> {
        &self.voids
    }

    pub fn walls(&self) -> &Vec<Position> {
        &self.walls
    }

    pub fn gems(&self) -> &Vec<Position> {
        &self.gems
    }

    pub fn sources(&self) -> &Vec<(Position, LaserConfig)> {
        &self.lasers
    }

    pub fn add_random_starts(&mut self, starts: Vec<Vec<Position>>) {
        for (i, start) in starts.into_iter().enumerate() {
            let start = self.filter_positions(start, &self.walls);
            while i >= self.random_starts.len() {
                self.random_starts.push(vec![]);
            }
            self.random_starts[i].extend(start);
        }
    }

    pub fn add_exits(&mut self, exits: Vec<Position>) {
        let exits = self.filter_positions(exits, &self.walls);
        self.exits.extend(exits);
    }

    pub fn add_gems(&mut self, gems: Vec<Position>) {
        let gems = self.filter_positions(gems, &self.walls);
        self.gems.extend(gems);
    }

    fn filter_positions(
        &self,
        positions: Vec<Position>,
        forbidden: &Vec<Position>,
    ) -> Vec<Position> {
        positions
            .into_iter()
            .filter(|pos| !forbidden.contains(pos))
            .collect()
    }

    pub fn to_world(mut self) -> Result<World, ParseError> {
        self.pre_validate()?;
        let (grid, lasers_positions) = self.make_grid();
        self.post_validate()?;
        let source_positions = self.lasers.iter().map(|(pos, _)| *pos).collect();
        Ok(World::new(
            grid,
            self.gems,
            self.random_starts,
            self.voids,
            self.exits,
            self.walls,
            source_positions,
            lasers_positions,
        ))
    }

    pub fn to_string(&self) -> String {
        if let Ok(string) = to_v1_string(&self) {
            return string;
        }
        let toml_config: TomlConfig = self.into();
        toml_config.to_toml_string()
    }

    fn pre_validate(&self) -> Result<(), ParseError> {
        // There are some agents
        if self.random_starts.is_empty() {
            return Err(ParseError::NoAgents);
        }
        // There are enough exit tiles
        if self.exits.len() < self.n_agents() {
            return Err(ParseError::NotEnoughExitTiles {
                n_starts: self.n_agents(),
                n_exits: self.exits.len(),
            });
        }

        // // Check that there are no lasers with an agent ID that does not exist
        // for (_, source) in self.lasers.iter() {
        //     if source.agent_id >= self.n_agents() {
        //         return Err(ParseError::InvalidLaserSourceAgentId {
        //             asked_id: source.agent_id,
        //             n_agents: self.n_agents(),
        //         });
        //     }
        // }

        Ok(())
    }

    fn post_validate(&self) -> Result<(), ParseError> {
        // All agents have at least one start tile
        for (agent_id, starts) in self.random_starts.iter().enumerate() {
            if starts.is_empty() {
                return Err(ParseError::AgentWithoutStart { agent_id });
            }
        }

        // Check that there are enough start tiles for all agents
        let n_starts = self
            .random_starts
            .iter()
            .fold(0usize, |sum, current| sum + current.len());
        if n_starts < self.n_agents() {
            return Err(ParseError::NotEnoughStartTiles {
                n_starts,
                n_agents: self.n_agents(),
            });
        }
        Ok(())
    }

    pub fn n_agents(&self) -> usize {
        self.random_starts.len()
    }

    fn make_grid(&mut self) -> (Vec<Vec<Tile>>, Vec<Position>) {
        let mut grid = Vec::with_capacity(self.height);
        for _ in 0..self.height {
            let mut row = Vec::with_capacity(self.width);
            for _ in 0..self.width {
                row.push(Tile::Floor { agent: None });
            }
            grid.push(row);
        }
        for pos in &self.gems {
            grid[pos.i][pos.j] = Tile::Gem(Gem::default());
        }
        for pos in &self.exits {
            grid[pos.i][pos.j] = Tile::Exit { agent: None };
        }
        for pos in &self.voids {
            grid[pos.i][pos.j] = Tile::Void(Void::default());
        }
        for pos in &self.walls {
            grid[pos.i][pos.j] = Tile::Wall;
        }
        let laser_positions = self.laser_setup(&mut grid).into_iter().collect();
        (grid, laser_positions)
    }

    /// Place the laser sources and wrap the required tiles behind a
    /// `Laser` tile.
    fn laser_setup(&mut self, grid: &mut Vec<Vec<Tile>>) -> HashSet<Position> {
        let mut laser_positions = HashSet::new();
        let width = grid[0].len() as i32;
        let height: i32 = grid.len() as i32;
        for (pos, source) in &self.lasers {
            let mut beam_positions = vec![];
            let delta = source.direction.delta();
            let (mut i, mut j) = (pos.i as i32, pos.j as i32);
            (i, j) = ((i + delta.0), (j + delta.1));
            while i >= 0 && j >= 0 && i < height && j < width {
                let pos = Position {
                    i: i as usize,
                    j: j as usize,
                };
                if !grid[pos.i][pos.j].is_walkable() {
                    break;
                }
                beam_positions.push(pos);
                (i, j) = ((i + delta.0), (j + delta.1));
            }
            laser_positions.extend(&beam_positions);
            let source = source.build(beam_positions.len());
            let mut is_blocked = false;
            for (i, pos) in beam_positions.into_iter().enumerate() {
                if let Some(agent_starts) = self.random_starts.get(source.agent_id()) {
                    if agent_starts.len() == 1 && agent_starts.contains(&pos) {
                        is_blocked = true;
                    }
                }
                let wrapped = grid[pos.i].remove(pos.j);
                let laser = Tile::Laser(Laser::new(wrapped, source.beam(), i));
                if !is_blocked {
                    // Remove the random starts on this location for agents of a different ID if the agent would die on reset
                    for (start_agent_id, starts) in self.random_starts.iter_mut().enumerate() {
                        if start_agent_id == source.agent_id() {
                            continue;
                        }
                        let len_before = starts.len();
                        starts.retain(|start| *start != pos);
                        if starts.len() != len_before {
                            eprintln!(
                                "[WARNING] {pos:?} is not a valid start position for agent {start_agent_id} since the agent would be killed on startup. The starting position {pos:?} has therefore been removed for agent {start_agent_id}."
                            );
                        }
                    }
                }

                grid[pos.i].insert(pos.j, laser);
            }
            grid[pos.i][pos.j] = Tile::LaserSource(source);
        }
        laser_positions
    }
}
