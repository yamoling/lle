use itertools::izip;
use std::{
    cell::Cell,
    fs::File,
    io::{BufReader, Read},
    rc::Rc,
};

use crate::{
    agent::Agent,
    reward_collector::RewardCollector,
    tiles::{laser::Laser, laser_source::LaserSource, Tile, TileType},
    utils::find_duplicates,
    Action, Position, WorldError,
};

#[derive(Clone)]
pub struct World {
    width: usize,
    height: usize,
    start_pos: Vec<Position>,
    exit_pos: Vec<Position>,
    grid: Vec<Vec<Tile>>,
    agents: Vec<Agent>,
    agent_positions: Vec<Position>,
    reward_collector: Rc<RewardCollector>,
}

/// Methods for Rust usage
impl World {
    fn new(tiles: Vec<Vec<TileType>>) -> Result<Self, WorldError> {
        if tiles.is_empty() {
            return Err(WorldError::EmptyWorld);
        }
        let mut world = World::setup(tiles)?;
        world.sanity_check()?;
        world.reset();
        Ok(world)
    }

    pub fn from_file(file: &str) -> Result<Self, WorldError> {
        let file = match File::open(file) {
            Ok(f) => f,
            Err(_) => {
                return Err(WorldError::InvalidFileName {
                    file_name: file.into(),
                })
            }
        };
        let mut reader = BufReader::new(file);
        let mut world_str = String::new();
        reader.read_to_string(&mut world_str).unwrap();
        World::try_from(world_str)
    }

    pub fn print(&self) {
        for row in self.grid.iter() {
            for tile in row.iter() {
                print!("{} ", tile);
            }
            println!();
        }
    }

    pub fn width(&self) -> usize {
        self.width
    }

    pub fn height(&self) -> usize {
        self.height
    }

    pub fn available_actions(&self) -> Vec<Vec<Action>> {
        let mut available_actions = vec![];
        for (agent, agent_pos) in self.agents.iter().zip(self.agent_positions.iter()) {
            let mut agent_actions = vec![Action::Stay];
            if !agent.has_arrived() {
                for action in [Action::North, Action::East, Action::South, Action::West] {
                    if let Ok(pos) = &action + agent_pos {
                        if let Some(tile) = self.get(pos) {
                            if tile.is_waklable() && !tile.is_occupied() {
                                agent_actions.push(action.clone());
                            }
                        }
                    }
                }
            }
            available_actions.push(agent_actions);
        }
        available_actions
    }

    fn solve_vertex_conflicts(new_pos: &mut [Position], old_pos: &[Position]) {
        let mut conflict = true;

        while conflict {
            conflict = false;
            let duplicates = find_duplicates(new_pos);
            for (i, is_duplicate) in duplicates.iter().enumerate() {
                if *is_duplicate {
                    conflict = true;
                    new_pos[i] = old_pos[i];
                }
            }
        }
    }

    /// Creates an iterator over all tiles in the grid with their (i, j) coordinates
    pub fn tiles(&self) -> impl Iterator<Item = ((u32, u32), &Tile)> {
        let mut res = vec![];
        for (i, row) in self.grid.iter().enumerate() {
            for (j, tile) in row.iter().enumerate() {
                res.push(((i as u32, j as u32), tile));
            }
        }
        res.into_iter()
    }

    fn setup(board: Vec<Vec<TileType>>) -> Result<World, WorldError> {
        let n_agents: u32 = board
            .iter()
            .flatten()
            .map(|t| if let TileType::Start { .. } = t { 1 } else { 0 })
            .sum();

        let reward_collector = Rc::new(RewardCollector::new(n_agents));
        let mut agents = vec![];
        let mut grid = vec![];
        let mut exit_pos = vec![];
        let mut gem_pos = vec![];
        let mut start_pos = vec![];
        let mut sources_pos = vec![];
        for (i, row) in board.into_iter().enumerate() {
            let mut grid_row: Vec<Tile> = vec![];
            for (j, tile) in row.into_iter().enumerate() {
                match tile {
                    TileType::Start { agent_num } => {
                        start_pos.push((agent_num, (i, j)));
                        agents.push(Agent::new(agent_num));
                    }
                    TileType::Exit { .. } => {
                        exit_pos.push((i, j));
                    }
                    TileType::Gem { .. } => gem_pos.push((i, j)),
                    TileType::LaserSource(source) => {
                        sources_pos.push((i, j, source));
                    }
                    _ => {}
                }
                grid_row.push(Tile::new(tile, reward_collector.clone()));
            }
            grid.push(grid_row);
        }
        // Sort agents and start tiles by agent number
        // Make sure there are no gaps in agent numbers
        agents.sort_by_key(|agent| agent.num());
        for (agent_num, agent) in agents.iter().enumerate() {
            assert_eq!(agent_num as u32, agent.num());
        }
        start_pos.sort_by_key(|(agent_num, _)| *agent_num);
        let start_pos: Vec<(usize, usize)> = start_pos.into_iter().map(|(_, pos)| pos).collect();
        let mut world = Self {
            width: grid[0].len(),
            height: grid.len(),
            start_pos: start_pos.clone(),
            exit_pos,
            grid,
            agents,
            agent_positions: start_pos,
            reward_collector,
        };
        for (i, j, source) in sources_pos {
            world.create_beam((i, j), source)?;
        }
        Ok(world)
    }

    fn create_beam(&mut self, pos: Position, source: LaserSource) -> Result<(), WorldError> {
        let (mut i, mut j) = pos;
        let mut states = vec![];
        let mut positions = vec![];
        let delta = source.direction().delta();

        (i, j) = ((i as i32 + delta.0) as usize, (j as i32 + delta.1) as usize);
        while let Some(tile) = self.get((i, j)) {
            match tile.tile_type() {
                // break if the beam hits a wall or another laser source
                TileType::Wall | TileType::LaserSource { .. } => break,
                // Otherwise, add the tile to the beam
                _ => {
                    positions.push((i, j));
                    states.push(Rc::new(Cell::new(true)));
                }
            }
            (i, j) = ((i as i32 + delta.0) as usize, (j as i32 + delta.1) as usize);
        }

        for (i, j) in positions {
            let replaced = self.grid[i].remove(j);
            if let TileType::Start { agent_num } = replaced.tile_type() {
                // Disable all next tiles if it crosses a start tile of the same colour
                if *agent_num == source.agent_num() {
                    states = states.iter().map(|_| Rc::new(Cell::new(false))).collect();
                }
                // Otherwise, the agent dies at startup if the beam is on-> return an error
                else if states[0].get() {
                    return Err(WorldError::AgentKilledOnStartup {
                        agent_num: *agent_num,
                        laser_num: source.agent_num(),
                        i,
                        j,
                    });
                }
            }

            // First item is the state of the current laser, the rest are the states of the following tiles
            let is_on = states.remove(0);
            let next_tiles_status = states.clone();

            let laser = Laser::new(
                source.direction(),
                source.agent_num(),
                Box::new(replaced),
                is_on,
                next_tiles_status,
            );
            let laser = Tile::new(TileType::Laser(laser), self.reward_collector.clone());
            self.grid[i].insert(j, laser);
        }
        Ok(())
    }

    fn get(&self, pos: Position) -> Option<&Tile> {
        let (i, j) = pos;
        if i >= self.height {
            return None;
        }
        if j >= self.width {
            return None;
        }
        Some(&self.grid[i][j])
    }

    fn sanity_check(&self) -> Result<(), WorldError> {
        if self.agents.is_empty() {
            return Err(WorldError::NoAgents);
        }
        if self.exit_pos.len() != self.start_pos.len() {
            return Err(WorldError::InconsistentNumberOfAgents {
                n_start_pos: self.start_pos.len(),
                n_exit_pos: self.exit_pos.len(),
            });
        }
        let width = self.width();
        for (i, row) in self.grid.iter().enumerate() {
            if row.len() != width {
                return Err(WorldError::InconsistentDimensions {
                    expected_n_cols: width,
                    actual_n_cols: row.len(),
                    row: i,
                });
            }
        }

        Ok(())
    }
    pub fn n_agents(&self) -> usize {
        self.agents.len()
    }

    pub fn agents(&self) -> &Vec<Agent> {
        &self.agents
    }

    pub fn gems_collected(&self) -> u32 {
        self.reward_collector.episode_gems_collected()
    }

    pub fn reset(&mut self) {
        self.reward_collector.reset();
        for row in self.grid.iter_mut() {
            for tile in row.iter_mut() {
                tile.reset();
            }
        }
        self.agent_positions = self.start_pos.clone();
        for ((i, j), agent) in self.agent_positions.iter().zip(self.agents.iter()) {
            self.grid[*i][*j].pre_enter(agent);
        }
        for ((i, j), agent) in self.agent_positions.iter().zip(self.agents.iter_mut()) {
            self.grid[*i][*j].enter(agent);
        }
    }

    /// Perform one step in the environment and return the corresponding reward.
    pub fn step(&mut self, actions: &[Action]) -> i32 {
        assert!(self.n_agents() == actions.len());

        // Available positions account for edge, following and swapping conflicts
        let available_actions = self.available_actions();
        for (action, availables) in actions.iter().zip(available_actions) {
            assert!(availables.contains(action));
        }
        let mut new_positions = self
            .agent_positions
            .iter()
            .zip(actions)
            .map(|(pos, action)| (action + pos).unwrap())
            .collect::<Vec<_>>();

        // Check for vertex conflicts
        // If a new_pos occurs more than once, then set it back to its original position
        World::solve_vertex_conflicts(&mut new_positions, &self.agent_positions);

        for (agent, old_pos, new_pos) in izip!(&self.agents, &self.agent_positions, &new_positions)
        {
            let (old_i, old_j) = *old_pos;
            self.grid[old_i][old_j].leave();
            let (new_i, new_j) = *new_pos;
            self.grid[new_i][new_j].pre_enter(agent);
        }

        for (agent, new_pos) in izip!(&mut self.agents, &new_positions) {
            let (i, j) = *new_pos;
            self.grid[i][j].enter(agent);
        }
        self.agent_positions = new_positions;
        self.reward_collector.consume_step_reward()
    }

    pub fn done(&self) -> bool {
        return self.agents.iter().any(|a| a.is_dead())
            || self.agents.iter().all(|a| a.has_arrived());
    }
}

impl TryFrom<String> for World {
    type Error = WorldError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        let tiles = parse(&value);
        World::new(tiles)
    }
}

impl TryFrom<&str> for World {
    type Error = WorldError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let tiles = parse(value);
        World::new(tiles)
    }
}

fn parse(world_str: &str) -> Vec<Vec<TileType>> {
    let mut res = vec![];
    for line in world_str.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let mut row = vec![];
        for token in line.split_whitespace() {
            row.push(TileType::from(token));
        }
        res.push(row);
    }
    res
}

#[cfg(test)]
#[path = "./unit_tests/test_world.rs"]
mod tests;
