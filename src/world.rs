use itertools::izip;
use std::{
    cell::Cell,
    fs::File,
    io::{BufReader, Read},
    rc::Rc,
};

use crate::{
    agent::{Agent, AgentId},
    errors::RuntimeWorldError,
    reward_collector::RewardCollector,
    tiles::{Direction, Exit, Floor, Gem, Laser, LaserBeam, LaserSource, Start, Tile, Wall},
    utils::find_duplicates,
    Action, Position, WorldError,
};

#[derive(Debug, Clone)]
pub struct World {
    width: usize,
    height: usize,
    start_tiles: Vec<Position>,
    grid: Vec<Vec<Box<dyn Tile>>>,
    agents: Vec<Agent>,
    agent_positions: Vec<Position>,
    reward_collector: Rc<RewardCollector>,
    available_actions: Vec<Vec<Action>>,
}

/// Methods for Rust usage
impl World {
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
                print!("{:?} ", tile);
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

    fn update_available_actions(&mut self) {
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
        self.available_actions = available_actions;
    }

    pub fn available_actions(&self) -> &Vec<Vec<Action>> {
        &self.available_actions
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
    pub fn tiles(&self) -> impl Iterator<Item = ((u32, u32), &Box<dyn Tile>)> {
        let mut res = vec![];
        for (i, row) in self.grid.iter().enumerate() {
            for (j, tile) in row.iter().enumerate() {
                res.push(((i as u32, j as u32), tile));
            }
        }
        res.into_iter()
    }

    // Remove unrelevant warning
    #[allow(clippy::borrowed_box)]
    fn get(&self, pos: Position) -> Option<&Box<dyn Tile>> {
        let (i, j) = pos;
        if i >= self.height {
            return None;
        }
        if j >= self.width {
            return None;
        }
        Some(&self.grid[i][j])
    }

    fn sanity_check(&self, n_exit_tiles: usize) -> Result<(), WorldError> {
        // There is at least one agent
        if self.agents.is_empty() {
            return Err(WorldError::NoAgents);
        }
        if self.start_tiles.len() > n_exit_tiles {
            return Err(WorldError::NotEnoughExitTiles {
                n_starts: self.start_tiles.len(),
                n_exits: n_exit_tiles,
            });
        }

        // All rows have the same length
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
        // Agents are ordered
        for (agent_num, agent) in self.agents.iter().enumerate() {
            assert_eq!(agent_num as u32, agent.id());
        }
        Ok(())
    }
    pub fn n_agents(&self) -> usize {
        self.agents.len()
    }

    pub fn agents(&self) -> &Vec<Agent> {
        &self.agents
    }

    pub fn agent_positions(&self) -> &Vec<Position> {
        &self.agent_positions
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
        self.agent_positions = self.start_tiles.clone();
        for ((i, j), agent) in self.agent_positions.iter().zip(self.agents.iter()) {
            self.grid[*i][*j].pre_enter(agent);
        }
        for ((i, j), agent) in self.agent_positions.iter().zip(self.agents.iter_mut()) {
            self.grid[*i][*j].enter(agent);
        }
        for agent in &mut self.agents {
            agent.reset();
        }
        self.update_available_actions();
    }

    /// Perform one step in the environment and return the corresponding reward.
    pub fn step(&mut self, actions: &[Action]) -> Result<i32, RuntimeWorldError> {
        assert!(self.n_agents() == actions.len());

        // Available positions account for edge, following and swapping conflicts
        for (agent_id, (action, availables)) in
            actions.iter().zip(&self.available_actions).enumerate()
        {
            if !availables.contains(action) {
                return Err(RuntimeWorldError::InvalidAction {
                    agent_id: agent_id as u32,
                    available: availables.clone(),
                    taken: action.clone(),
                });
            }
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

        for (agent, action, old_pos, new_pos) in
            izip!(&self.agents, actions, &self.agent_positions, &new_positions)
        {
            if *action != Action::Stay {
                let (old_i, old_j) = *old_pos;
                self.grid[old_i][old_j].leave();
                let (new_i, new_j) = *new_pos;
                self.grid[new_i][new_j].pre_enter(agent);
            }
        }

        for (agent, action, new_pos) in izip!(&mut self.agents, actions, &new_positions) {
            if *action != Action::Stay {
                let (i, j) = *new_pos;
                self.grid[i][j].enter(agent);
            }
        }
        self.agent_positions = new_positions;
        self.update_available_actions();
        Ok(self.reward_collector.consume_step_reward())
    }

    pub fn done(&self) -> bool {
        return self.agents.iter().any(|a| a.is_dead())
            || self.agents.iter().all(|a| a.has_arrived());
    }
}

impl TryFrom<String> for World {
    type Error = WorldError;

    fn try_from(world_str: String) -> Result<Self, Self::Error> {
        parse(&world_str)
    }
}

impl TryFrom<&str> for World {
    type Error = WorldError;

    fn try_from(world_str: &str) -> Result<Self, Self::Error> {
        parse(world_str)
    }
}

/// Wrap the tiles behin lasers with a `Laser` tile.
fn laser_setup(
    mut grid: Vec<Vec<Box<dyn Tile>>>,
    laser_sources: Vec<(Position, Direction, AgentId)>,
) -> Vec<Vec<Box<dyn Tile>>> {
    let width = grid[0].len() as i32;
    let height = grid.len() as i32;
    for (pos, dir, agent_num) in laser_sources {
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
            beam.push(Rc::new(Cell::new(true)));
            beam_pos.push(pos);
            (i, j) = ((i + delta.0), (j + delta.1));
        }

        for (i, pos) in beam_pos.iter().enumerate() {
            let beam = LaserBeam::new(beam[i..].to_vec());
            let wrapped = grid[pos.0].remove(pos.1);
            let laser = Box::new(Laser::new(agent_num, dir, wrapped, beam));
            grid[pos.0].insert(pos.1, laser);
        }
    }
    grid
}

fn parse(world_str: &str) -> Result<World, WorldError> {
    let mut grid = vec![];
    let mut start_tiles: Vec<(Position, AgentId)> = vec![];
    let mut exit_tiles: Vec<Position> = vec![];
    let mut laser_sources: Vec<(Position, Direction, AgentId)> = vec![];
    for line in world_str.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let i = grid.len();
        let mut row = vec![];
        for (j, token) in line.split_whitespace().enumerate() {
            let tile: Box<dyn Tile> = match token.to_uppercase().chars().next().unwrap() {
                '.' => Box::<Floor>::default(),
                '@' => Box::new(Wall::new()),
                'G' => Box::<Gem>::default(),
                'S' => {
                    let agent_num = token[1..].parse().unwrap();
                    start_tiles.push(((i, j), agent_num));
                    Box::new(Start::new(agent_num))
                }
                'X' => {
                    exit_tiles.push((i, j));
                    Box::<Exit>::default()
                }
                'L' => {
                    let direction = Direction::try_from(&token[2..]).unwrap();
                    let agent_num = token[1..2].parse().unwrap();
                    laser_sources.push(((i, j), direction, agent_num));
                    Box::new(LaserSource::new(direction, agent_num))
                }
                other => {
                    return Err(WorldError::InvalidTile {
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
        return Err(WorldError::EmptyWorld);
    }
    grid = laser_setup(grid, laser_sources);

    // Sort start tiles by agent id, then map to positions
    start_tiles.sort_by_key(|(_, id)| *id);
    let start_tiles: Vec<Position> = start_tiles.iter().map(|(pos, _)| *pos).collect();
    // Initial agent positions are start tiles
    let agent_positions = start_tiles.clone();
    let reward_collector = Rc::new(RewardCollector::new(start_tiles.len() as u32));
    let agents = start_tiles
        .iter()
        .enumerate()
        .map(|(id, _)| Agent::new(id as u32, reward_collector.clone()))
        .collect();

    let world = World {
        width: grid[0].len(),
        height: grid.len(),
        agent_positions,
        start_tiles,
        agents,
        grid,
        reward_collector,
        available_actions: vec![],
    };
    world.sanity_check(exit_tiles.len())?;
    Ok(world)
}

#[cfg(test)]
#[path = "./unit_tests/test_world.rs"]
mod tests;
