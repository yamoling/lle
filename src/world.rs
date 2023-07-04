use itertools::{izip, Itertools};
use std::{
    cell::Cell,
    collections::HashMap,
    fs::File,
    io::{BufReader, Read},
    rc::Rc,
};

use crate::{
    agent::Agent,
    errors::RuntimeWorldError,
    levels,
    reward_collector::RewardCollector,
    tiles::{Direction, Exit, Floor, Gem, Laser, LaserBeam, LaserSource, Start, Tile, Wall},
    utils::find_duplicates,
    Action, Position, WorldError,
};

#[derive(Debug, Clone)]
pub struct World {
    width: usize,
    height: usize,

    grid: Vec<Vec<Rc<dyn Tile>>>,
    gems: HashMap<Position, Rc<Gem>>,
    lasers: Vec<(Position, Rc<Laser>)>,
    walls: Vec<Position>,
    sources: HashMap<Position, Rc<LaserSource>>,
    starts: HashMap<Position, Rc<Start>>,
    exits: HashMap<Position, Rc<Exit>>,

    agents: Vec<Agent>,
    start_positions: Vec<Position>,
    agent_positions: Vec<Position>,
    reward_collector: Rc<RewardCollector>,
    available_actions: Vec<Vec<Action>>,
    done: bool,
}

/// Methods for Rust usage
impl World {
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

    pub fn n_gems(&self) -> usize {
        self.gems.len()
    }

    pub fn available_actions(&self) -> &Vec<Vec<Action>> {
        &self.available_actions
    }

    pub fn width(&self) -> usize {
        self.width
    }

    pub fn height(&self) -> usize {
        self.height
    }

    pub fn done(&self) -> bool {
        self.done
    }

    pub fn walls(&self) -> impl Iterator<Item = &Position> {
        self.walls.iter()
    }

    pub fn gems(&self) -> impl Iterator<Item = (&Position, &Gem)> {
        self.gems.iter().map(|(pos, gem)| (pos, gem.as_ref()))
    }

    pub fn laser_sources(&self) -> impl Iterator<Item = (&Position, &LaserSource)> {
        self.sources
            .iter()
            .map(|(pos, source)| (pos, source.as_ref()))
    }

    pub fn starts(&self) -> impl Iterator<Item = (&Position, &Start)> {
        self.starts.iter().map(|(pos, start)| (pos, start.as_ref()))
    }

    pub fn exits(&self) -> impl Iterator<Item = &Position> {
        self.exits.keys()
    }

    pub fn lasers(&self) -> impl Iterator<Item = (&Position, &Laser)> {
        self.lasers.iter().map(|(pos, laser)| (pos, laser.as_ref()))
    }

    fn update(&mut self) {
        let mut available_actions = vec![];
        for (agent, agent_pos) in self.agents.iter().zip(self.agent_positions.iter()) {
            let mut agent_actions = vec![Action::Stay];
            if !agent.has_arrived() {
                for action in [Action::North, Action::East, Action::South, Action::West] {
                    if let Ok(pos) = &action + agent_pos {
                        if let Some(tile) = self.at(pos) {
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
        self.done =
            self.agents.iter().any(|a| a.is_dead()) || self.agents.iter().all(|a| a.has_arrived());
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
    pub fn tiles(&self) -> impl Iterator<Item = ((u32, u32), &dyn Tile)> {
        let mut res = vec![];
        for (i, row) in self.grid.iter().enumerate() {
            for (j, tile) in row.iter().enumerate() {
                res.push(((i as u32, j as u32), tile.as_ref()));
            }
        }
        res.into_iter()
    }

    pub fn at(&self, pos: Position) -> Option<&dyn Tile> {
        let (i, j) = pos;
        if i >= self.height {
            return None;
        }
        if j >= self.width {
            return None;
        }
        Some(self.grid[i][j].as_ref())
    }

    pub fn reset(&mut self) {
        self.reward_collector.reset();
        for row in self.grid.iter_mut() {
            for tile in row.iter_mut() {
                tile.reset();
            }
        }
        self.agent_positions = self.start_positions.clone();
        for ((i, j), agent) in self.agent_positions.iter().zip(self.agents.iter()) {
            self.grid[*i][*j].pre_enter(agent);
        }
        for ((i, j), agent) in self.agent_positions.iter().zip(self.agents.iter_mut()) {
            self.grid[*i][*j].enter(agent);
        }
        for agent in &mut self.agents {
            agent.reset();
        }
        self.update();
    }

    /// Perform one step in the environment and return the corresponding reward.
    pub fn step(&mut self, actions: &[Action]) -> Result<i32, RuntimeWorldError> {
        if self.done {
            return Err(RuntimeWorldError::WorldIsDone);
        }
        if self.n_agents() != actions.len() {
            return Err(RuntimeWorldError::InvalidNumberOfActions {
                given: actions.len(),
                expected: self.n_agents(),
            });
        }

        // Available positions account for edge, following and swapping conflicts
        for (agent_id, (action, availables)) in
            actions.iter().zip(&self.available_actions).enumerate()
        {
            if !availables.contains(action) {
                return Err(RuntimeWorldError::InvalidAction {
                    agent_id,
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
        self.update();
        Ok(self.reward_collector.consume_step_reward())
    }

    pub fn force_state(
        &mut self,
        agent_positions: Vec<Position>,
        gem_collected: &[bool],
    ) -> Result<(), RuntimeWorldError> {
        if gem_collected.len() != self.n_gems() {
            return Err(RuntimeWorldError::InvalidNumberOfGems {
                given: gem_collected.len(),
                expected: self.gems.len(),
            });
        }
        if agent_positions.len() != self.n_agents() {
            return Err(RuntimeWorldError::InvalidNumberOfAgents {
                given: agent_positions.len(),
                expected: self.n_agents(),
            });
        }

        // Reset tiles and agents (but do not enter the new tiles)
        for row in &mut self.grid {
            for tile in row {
                tile.reset();
            }
        }
        for agent in &mut self.agents {
            agent.reset();
        }
        // Set the gem states
        for (gem, collect) in izip!(self.gems.values(), gem_collected) {
            if *collect {
                gem.collect();
            }
        }

        self.agent_positions = agent_positions;
        for ((i, j), agent) in self.agent_positions.iter().zip(self.agents.iter()) {
            self.grid[*i][*j].pre_enter(agent);
        }
        for ((i, j), agent) in self.agent_positions.iter().zip(self.agents.iter_mut()) {
            self.grid[*i][*j].enter(agent);
        }
        self.update();
        Ok(())
    }
}

// Creational methods
impl World {
    fn sanity_check(&self) -> Result<(), WorldError> {
        // There is at least one agent
        if self.agents.is_empty() {
            return Err(WorldError::NoAgents);
        }
        if self.starts.len() > self.exits.len() {
            return Err(WorldError::NotEnoughExitTiles {
                n_starts: self.starts.len(),
                n_exits: self.exits.len(),
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
            assert_eq!(agent_num, agent.id());
        }
        Ok(())
    }

    pub fn from_file(file: &str) -> Result<Self, WorldError> {
        if let Some(world_str) = levels::get_level(file) {
            return World::try_from(world_str);
        }
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
    grid: &mut Vec<Vec<Rc<dyn Tile>>>,
    laser_sources: &HashMap<Position, Rc<LaserSource>>,
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

fn parse(world_str: &str) -> Result<World, WorldError> {
    let mut grid = vec![];
    let mut gems: HashMap<Position, Rc<Gem>> = HashMap::new();
    let mut starts: HashMap<Position, Rc<Start>> = HashMap::new();
    let mut exits: HashMap<Position, Rc<Exit>> = HashMap::new();
    let mut walls: Vec<Position> = vec![];
    let mut sources: HashMap<Position, Rc<LaserSource>> = HashMap::new();
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
                    gems.insert((i, j), gem.clone());
                    gem
                }
                'S' => {
                    let agent_num = token[1..].parse().unwrap();
                    let start = Rc::new(Start::new(agent_num));
                    starts.insert((i, j), start.clone());
                    start
                }
                'X' => {
                    let exit = Rc::<Exit>::default();
                    exits.insert((i, j), exit.clone());
                    exit
                }
                'L' => {
                    let direction = Direction::try_from(&token[2..]).unwrap();
                    let agent_num = token[1..2].parse().unwrap();
                    let source = Rc::new(LaserSource::new(direction, agent_num));
                    sources.insert((i, j), source.clone());
                    source
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
    let lasers = laser_setup(&mut grid, &sources);

    let agent_positions: Vec<Position> = starts
        .iter()
        .map(|(pos, start)| (pos, start.agent_id()))
        .sorted_by_key(|(_, id)| *id)
        .map(|(pos, _)| *pos)
        .collect();
    let start_positions = agent_positions.clone();

    let reward_collector = Rc::new(RewardCollector::new(starts.len() as u32));
    let agents = starts
        .iter()
        .enumerate()
        .map(|(id, _)| Agent::new(id as u32, reward_collector.clone()))
        .collect();

    let world = World {
        width: grid[0].len(),
        height: grid.len(),
        agent_positions,
        starts,
        walls,
        agents,
        exits,
        gems,
        sources,
        grid,
        lasers,
        start_positions,
        reward_collector,
        available_actions: vec![],
        done: false,
    };
    world.sanity_check()?;
    Ok(world)
}

#[cfg(test)]
#[path = "./unit_tests/test_world.rs"]
mod tests;
