/// The core logic of LLE, which should not be parametrisable.
use itertools::{izip, Itertools};
use std::{
    fs::File,
    io::{BufReader, Read},
};

use crate::{
    agent::Agent,
    core::{levels, parsing::parse},
    tiles::{Gem, Laser, LaserSource, Start, Tile},
    utils::{find_duplicates, sample_different},
    Action, AgentId, ParseError, Position, RuntimeWorldError, WorldEvent, WorldState,
};

type JointAction = Vec<Action>;

pub struct World {
    width: usize,
    height: usize,
    world_string: String,

    grid: Vec<Vec<Tile>>,
    agents: Vec<Agent>,
    laser_source_positions: Vec<Position>,
    lasers_positions: Vec<Position>,
    gems_positions: Vec<Position>,
    /// Possible random start position of each agent.
    random_start_positions: Vec<Vec<Position>>,
    void_positions: Vec<Position>,
    exits: Vec<Position>,
    agents_positions: Vec<Position>,
    wall_positions: Vec<Position>,

    available_actions: Vec<Vec<Action>>,
    /// The actual start position of the agents since the last `reset`.
    start_positions: Vec<Position>,
    rng: rand::rngs::StdRng,
}

impl World {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        grid: Vec<Vec<Tile>>,
        gem_positions: Vec<Position>,
        random_start_positions: Vec<Vec<Position>>,
        void_positions: Vec<Position>,
        exit_positions: Vec<Position>,
        walls_positions: Vec<Position>,
        source_positions: Vec<Position>,
        lasers_positions: Vec<Position>,
        world_str: &str,
    ) -> Self {
        let agents = random_start_positions
            .iter()
            .enumerate()
            .map(|(id, _)| Agent::new(id as AgentId))
            .collect();
        let mut w = Self {
            width: grid[0].len(),
            height: grid.len(),
            gems_positions: gem_positions,
            agents_positions: vec![],
            random_start_positions,
            wall_positions: walls_positions,
            void_positions,
            agents,
            exits: exit_positions,
            grid,
            start_positions: vec![],
            world_string: world_str.into(),
            available_actions: vec![],
            laser_source_positions: source_positions,
            lasers_positions,
            rng: rand::SeedableRng::from_entropy(),
        };
        w.reset();
        w
    }

    pub fn n_agents(&self) -> usize {
        self.agents.len()
    }

    /// The world string upon which the world has been created.
    pub fn initial_world_string(&self) -> &str {
        &self.world_string
    }

    pub fn seed(&mut self, seed: u64) {
        self.rng = rand::SeedableRng::seed_from_u64(seed);
    }

    /// The current world string, taking into account the fact that some tiles may have changed (laser direction or colour).
    pub fn compute_world_string(&self) -> String {
        let mut str_tiles = vec![];
        for row in &self.grid {
            let mut str_row = vec![];
            for tile in row {
                str_row.push(tile.to_file_string());
            }
            str_row.push("\n".into());
            str_tiles.push(str_row);
        }
        for (agent_id, pos) in self.start_positions.iter().enumerate() {
            str_tiles[pos.i][pos.j] = Tile::Start(Start::new(agent_id)).to_file_string();
        }
        str_tiles
            .iter()
            .map(|row| row.join(" "))
            .collect::<Vec<String>>()
            .join("\n")
    }

    pub fn agents(&self) -> &Vec<Agent> {
        &self.agents
    }

    pub fn agents_positions(&self) -> &Vec<Position> {
        &self.agents_positions
    }

    pub fn gems_positions(&self) -> Vec<Position> {
        self.gems_positions.clone()
    }

    pub fn gems(&self) -> Vec<&Gem> {
        // Important: gems can be wrapped into lasers !
        self.gems_positions
            .iter()
            .map(|pos| match &self.grid[pos.i][pos.j] {
                Tile::Gem(gem) => gem,
                Tile::Laser(laser) => laser.gem().unwrap(),
                _ => unreachable!(),
            })
            .collect()
    }

    pub fn sources(&self) -> Vec<(Position, &LaserSource)> {
        self.laser_source_positions
            .iter()
            .map(|pos| {
                if let Tile::LaserSource(source) = &self.grid[pos.i][pos.j] {
                    (pos.clone(), source)
                } else {
                    unreachable!()
                }
            })
            .collect()
    }

    pub fn lasers(&self) -> Vec<(Position, &Laser)> {
        let mut lasers = vec![];
        for pos in &self.lasers_positions {
            if let Tile::Laser(laser) = &self.grid[pos.i][pos.j] {
                lasers.push((pos.clone(), laser));
                if let Tile::Laser(wrapped) = laser.wrapped() {
                    lasers.push((pos.clone(), wrapped));
                }
            } else {
                unreachable!()
            }
        }
        lasers
    }

    pub fn exits_positions(&self) -> Vec<Position> {
        self.exits.clone()
    }

    pub fn n_gems(&self) -> usize {
        self.gems_positions.len()
    }

    /// The available actions for each agent.
    /// The actions available to agent `n` are located in `world.available_actions()[n]`.
    pub fn available_actions(&self) -> &Vec<Vec<Action>> {
        &self.available_actions
    }

    /// Compute the available joint actions for all agents.
    /// The joint actions are all the possible combinations of the available actions for each agent.
    /// The result is a matrix of shape (x, n_agents) where x is the number of joint actions.
    pub fn available_joint_actions(&self) -> Vec<JointAction> {
        self.available_actions
            .clone()
            .into_iter()
            .multi_cartesian_product()
            .collect()
    }

    pub fn n_gems_collected(&self) -> usize {
        let mut res = 0;
        for pos in &self.gems_positions {
            if let Tile::Gem(gem) = &self.grid[pos.i][pos.j] {
                if gem.is_collected() {
                    res += 1;
                }
            }
        }
        res
    }

    pub fn n_agents_arrived(&self) -> usize {
        self.agents.iter().filter(|&a| a.has_arrived()).count()
    }

    pub fn width(&self) -> usize {
        self.width
    }

    pub fn height(&self) -> usize {
        self.height
    }

    pub fn walls(&self) -> Vec<Position> {
        self.wall_positions.clone()
    }

    pub fn starts(&self) -> Vec<Position> {
        self.start_positions.clone()
    }

    pub fn possible_starts(&self) -> Vec<Vec<Position>> {
        self.random_start_positions.clone()
    }

    pub fn void_positions(&self) -> Vec<Position> {
        self.void_positions.clone()
    }

    fn compute_available_actions(&self) -> Vec<Vec<Action>> {
        let mut available_actions = vec![];
        for (agent, agent_pos) in izip!(&self.agents, &self.agents_positions) {
            let mut agent_actions = vec![Action::Stay];
            if agent.is_alive() && !agent.has_arrived() {
                for action in [Action::North, Action::East, Action::South, Action::West] {
                    if let Ok(pos) = &action + agent_pos {
                        if let Some(tile) = self.at(&pos) {
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
    pub fn tiles(&self) -> Vec<(Position, &Tile)> {
        let mut res = vec![];
        for (i, row) in self.grid.iter().enumerate() {
            for (j, tile) in row.iter().enumerate() {
                res.push((Position { i, j }, tile));
            }
        }
        res
    }

    pub fn at(&self, pos: &Position) -> Option<&Tile> {
        if pos.i >= self.height {
            return None;
        }
        if pos.j >= self.width {
            return None;
        }
        Some(&self.grid[pos.i][pos.j])
    }

    pub fn at_mut(&mut self, pos: &Position) -> Option<&mut Tile> {
        if pos.i >= self.height {
            return None;
        }
        if pos.j >= self.width {
            return None;
        }
        Some(&mut self.grid[pos.i][pos.j])
    }

    pub fn reset(&mut self) {
        for row in self.grid.iter_mut() {
            for tile in row.iter_mut() {
                tile.reset();
            }
        }
        self.start_positions = sample_different(&mut self.rng, &self.random_start_positions);
        self.agents_positions = self.start_positions.clone();
        for (pos, agent) in izip!(&self.agents_positions, &self.agents) {
            self.grid[pos.i][pos.j]
                .pre_enter(agent)
                .expect("The agent should be able to pre-enter");
        }
        for (pos, agent) in izip!(&self.agents_positions, &mut self.agents) {
            self.grid[pos.i][pos.j].enter(agent);
        }
        for agent in &mut self.agents {
            agent.reset();
        }
        self.available_actions = self.compute_available_actions();
    }

    /// Perform one step in the environment and return the corresponding reward.
    pub fn step(&mut self, actions: &[Action]) -> Result<Vec<WorldEvent>, RuntimeWorldError> {
        if self.n_agents() != actions.len() {
            return Err(RuntimeWorldError::InvalidNumberOfActions {
                given: actions.len(),
                expected: self.n_agents(),
            });
        }

        // Available positions account for edge, following and swapping conflicts
        for (agent_id, (action, availables)) in izip!(actions, &self.available_actions).enumerate()
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
            .agents_positions
            .iter()
            .zip(actions)
            .map(|(pos, action)| (action + pos))
            .collect::<Result<Vec<Position>, RuntimeWorldError>>()?;

        // Check for vertex conflicts
        // If a new_pos occurs more than once, then set it back to its original position
        World::solve_vertex_conflicts(&mut new_positions, &self.agents_positions);
        let (mut events, mut agent_died) = self.move_agents(&new_positions)?;
        self.agents_positions = new_positions.clone();
        // At this stage, all agents are on their new positions.
        // However, some events (death) could still happen if an agent has died.
        while agent_died {
            let (additional_events, died2) = self.move_agents(&new_positions)?;
            events = [events, additional_events].concat();
            agent_died = died2;
        }
        self.available_actions = self.compute_available_actions();
        Ok(events)
    }

    fn move_agents(
        &mut self,
        new_positions: &[Position],
    ) -> Result<(Vec<WorldEvent>, bool), RuntimeWorldError> {
        // Leave old position
        for (agent, pos) in izip!(&self.agents, &self.agents_positions) {
            if agent.is_alive() {
                self.grid[pos.i][pos.j].leave();
            }
        }
        // Pre-enter
        for (agent, pos) in izip!(&self.agents, new_positions) {
            self.grid[pos.i][pos.j]
                .pre_enter(agent)
                .expect("When moving agents, the pre-enter should not fail");
        }
        // Enter
        let mut events = vec![];
        let mut agent_died = false;
        for (agent, pos) in izip!(&mut self.agents, new_positions) {
            if let Some(event) = self.grid[pos.i][pos.j].enter(agent) {
                if let WorldEvent::AgentDied { .. } = event {
                    agent_died = true;
                }
                events.push(event);
            }
        }
        Ok((events, agent_died))
    }

    pub fn get_state(&self) -> WorldState {
        WorldState {
            agents_positions: self.agents_positions.clone(),
            gems_collected: self.gems().iter().map(|gem| gem.is_collected()).collect(),
            agents_alive: self.agents.iter().map(|agent| agent.is_alive()).collect(),
        }
    }

    pub fn set_state(&mut self, state: &WorldState) -> Result<Vec<WorldEvent>, RuntimeWorldError> {
        if state.gems_collected.len() != self.n_gems() {
            return Err(RuntimeWorldError::InvalidNumberOfGems {
                given: state.gems_collected.len(),
                expected: self.gems_positions.len(),
            });
        }
        if state.agents_positions.len() != self.n_agents() {
            return Err(RuntimeWorldError::InvalidNumberOfAgents {
                given: state.agents_positions.len(),
                expected: self.n_agents(),
            });
        }
        // If any position is present twice, then the state is invalid
        if find_duplicates(&state.agents_positions).iter().any(|&b| b) {
            return Err(RuntimeWorldError::InvalidWorldState {
                reason: "There are two agents at the same position".into(),
                state: state.clone(),
            });
        }

        for pos in &state.agents_positions {
            if pos.i >= self.height || pos.j >= self.width {
                return Err(RuntimeWorldError::OutOfWorldPosition {
                    position: pos.clone(),
                });
            }
        }
        let current_state = self.get_state();

        // Reset tiles and agents (but do not enter the new tiles)
        for row in &mut self.grid {
            for tile in row {
                tile.reset();
            }
        }
        for (agent, alive) in &mut self.agents.iter_mut().zip(state.agents_alive.iter()) {
            agent.reset();
            if !alive {
                agent.die();
            }
        }
        // Collect the necessary gems BEFORE entering the tiles with the agents
        for (pos, &collect) in izip!(&self.gems_positions, &state.gems_collected) {
            if collect {
                if let Tile::Gem(gem) = &mut self.grid[pos.i][pos.j] {
                    gem.collect();
                }
            }
        }
        for (pos, agent) in izip!(&state.agents_positions, &self.agents) {
            if let Err(error) = self.grid[pos.i][pos.j].pre_enter(agent) {
                let reason = match error {
                    RuntimeWorldError::TileNotWalkable => "The tile is not walkable",
                    _ => "Unknown reason",
                }
                .into();
                // Reset the state to the one before the pre-enter
                self.set_state(&current_state).unwrap();
                return Err(RuntimeWorldError::InvalidAgentPosition {
                    position: pos.clone(),
                    reason,
                });
            }
        }
        // Set the agents positions after the pre-enter in case it fails
        self.agents_positions = state.agents_positions.to_vec();
        let mut events = vec![];
        for (pos, agent) in izip!(&self.agents_positions, &mut self.agents) {
            if let Some(event) = self.grid[pos.i][pos.j].enter(agent) {
                events.push(event);
            }
        }
        self.available_actions = self.compute_available_actions();
        Ok(events)
    }

    pub fn get_level(level: usize) -> Result<Self, ParseError> {
        let content = levels::LEVELS
            .get(level - 1)
            .ok_or(ParseError::InvalidLevel {
                asked: level,
                min: 1,
                max: levels::LEVELS.len(),
            })?;
        Self::try_from(content.to_string())
    }

    pub fn from_file(file: &str) -> Result<Self, ParseError> {
        if let Some(world_str) = levels::get_level_str(file) {
            return World::try_from(world_str);
        }
        let file = match File::open(file) {
            Ok(f) => f,
            Err(_) => {
                return Err(ParseError::InvalidFileName {
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
    type Error = ParseError;

    fn try_from(world_str: String) -> Result<Self, Self::Error> {
        parse(&world_str)
    }
}

impl TryFrom<&str> for World {
    type Error = ParseError;

    fn try_from(world_str: &str) -> Result<Self, Self::Error> {
        parse(world_str)
    }
}

impl Clone for World {
    fn clone(&self) -> Self {
        let mut world = Self::try_from(self.world_string.clone()).unwrap();
        let state = self.get_state();
        world.set_state(&state).unwrap();
        world
    }
}

#[cfg(test)]
mod test;
