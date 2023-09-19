use std::{cell::Cell, rc::Rc};

use crate::{
    agent::Agent,
    reward_collector::SharedRewardCollector,
    tiles::{Direction, LaserBeam},
    AgentId, Exit, Floor, Gem, Laser, LaserSource, Position, Start, Tile, Wall, World,
};

use super::errors::ParseError;

pub struct Parser {
    grid: Vec<Vec<Rc<dyn Tile>>>,
    gems: Vec<(Position, Rc<Gem>)>,
    start_positions: Vec<Position>,
    exit_positions: Vec<Position>,
    walls: Vec<Position>,
    sources: Vec<(Position, Rc<LaserSource>)>,
    lasers: Vec<(Position, Rc<Laser>)>,
    world_str: String,
}

impl Parser {
    pub fn new(world_str: &str) -> Result<Self, ParseError> {
        let parser = Self::parse(world_str)?;
        parser.sanity_check()?;
        Ok(parser)
    }

    pub fn n_agents(&self) -> usize {
        self.start_positions.len()
    }

    pub fn shared_world(self) -> World {
        // Create agents
        let reward_collector =
            Rc::new(SharedRewardCollector::new(self.start_positions.len() as u32));
        let agents = self
            .start_positions
            .iter()
            .enumerate()
            .map(|(id, _)| Agent::new(id as u32, reward_collector.clone()))
            .collect();

        World::new(
            self.grid,
            self.gems,
            self.lasers,
            self.sources,
            agents,
            self.start_positions,
            self.exit_positions,
            self.walls,
            reward_collector,
            &self.world_str,
        )
    }

    fn parse(world_str: &str) -> Result<Self, ParseError> {
        let mut grid = vec![];
        let mut gems: Vec<(Position, Rc<Gem>)> = vec![];
        let mut start_positions: Vec<(AgentId, Position)> = vec![];
        let mut exit_positions: Vec<Position> = vec![];
        let mut walls: Vec<Position> = vec![];
        let mut sources: Vec<(Position, Rc<LaserSource>)> = vec![];
        for line in world_str.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            let i = grid.len();
            let mut row = vec![];
            for (j, token) in line.split_whitespace().enumerate() {
                let tile: Rc<dyn Tile> = match token.to_uppercase().chars().next().unwrap() {
                    '.' => Rc::<Floor>::default(),
                    '@' => {
                        walls.push((i, j));
                        Rc::new(Wall::default())
                    }
                    'G' => {
                        let gem = Rc::<Gem>::default();
                        gems.push(((i, j), gem.clone()));
                        gem
                    }
                    'S' => {
                        let agent_id = token[1..].parse().unwrap();
                        // Check for duplicate agent ids
                        for (id, other_pos) in &start_positions {
                            if *id == agent_id {
                                return Err(ParseError::DuplicateStartTile {
                                    agent_id,
                                    start1: *other_pos,
                                    start2: (i, j),
                                });
                            }
                        }
                        start_positions.push((agent_id, (i, j)));
                        Rc::new(Start::new(agent_id))
                    }
                    'X' => {
                        let exit = Rc::<Exit>::default();
                        exit_positions.push((i, j));
                        exit
                    }
                    'L' => {
                        let direction = Direction::try_from(&token[2..]).unwrap();
                        let agent_num = token[1..2].parse().unwrap();
                        let source = Rc::new(LaserSource::new(direction, agent_num));
                        sources.push(((i, j), source.clone()));
                        source
                    }
                    other => {
                        return Err(ParseError::InvalidTile {
                            tile_str: other.into(),
                            line: i,
                            col: j,
                        });
                    }
                };
                row.push(tile);
            }
            grid.push(row);
        }
        if grid.is_empty() {
            return Err(ParseError::EmptyWorld);
        }
        // Sort start positions
        start_positions.sort_by(|(id_a, _), (id_b, _)| id_a.cmp(id_b));
        let start_positions: Vec<Position> = start_positions.iter().map(|(_, pos)| *pos).collect();

        Ok(Self {
            lasers: laser_setup(&mut grid, &sources),
            grid,
            gems,
            start_positions,
            exit_positions,
            walls,
            sources,
            world_str: world_str.into(),
        })
    }

    fn sanity_check(&self) -> Result<(), ParseError> {
        if self.start_positions.is_empty() {
            return Err(ParseError::NoAgents);
        }
        // There are enough start/exit tiles
        if self.start_positions.len() != self.exit_positions.len() {
            return Err(ParseError::NotEnoughExitTiles {
                n_starts: self.start_positions.len(),
                n_exits: self.exit_positions.len(),
            });
        }

        // All rows have the same length
        let width = self.grid[0].len();
        for (i, row) in self.grid.iter().enumerate() {
            if row.len() != width {
                return Err(ParseError::InconsistentDimensions {
                    expected_n_cols: width,
                    actual_n_cols: row.len(),
                    row: i,
                });
            }
        }
        Ok(())
    }
}

/// Wrap the tiles behind lasers with a `Laser` tile.
fn laser_setup(
    grid: &mut Vec<Vec<Rc<dyn Tile>>>,
    laser_sources: &[(Position, Rc<LaserSource>)],
) -> Vec<(Position, Rc<Laser>)> {
    let mut lasers = vec![];
    let width = grid[0].len() as i32;
    let height: i32 = grid.len() as i32;
    for (pos, source) in laser_sources.iter() {
        let dir = source.direction();
        let delta = dir.delta();
        let (mut i, mut j) = (pos.0 as i32, pos.1 as i32);
        let mut beam = vec![];
        let mut beam_pos = vec![];

        (i, j) = ((i + delta.0), (j + delta.1));
        while i >= 0 && j >= 0 && i < height && j < width {
            let pos = (i as usize, j as usize);
            if !grid[pos.0][pos.1].is_waklable() {
                break;
            }
            let status = Rc::new(Cell::new(true));
            beam.push(status.clone());
            beam_pos.push(pos);
            (i, j) = ((i + delta.0), (j + delta.1));
        }

        for (i, pos) in beam_pos.iter().enumerate() {
            let beam = LaserBeam::new(beam[i..].to_vec());
            let wrapped = grid[pos.0].remove(pos.1);
            let laser = Rc::new(Laser::new(source.agent_id(), dir, wrapped, beam));
            lasers.push((*pos, laser.clone()));
            grid[pos.0].insert(pos.1, laser);
        }
    }
    lasers
}
