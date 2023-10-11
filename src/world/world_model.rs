use itertools::izip;
use std::{
    fs::File,
    io::{BufReader, Read},
    rc::Rc,
};

use crate::{
    agent::Agent,
    parsing::{parse, ParseError},
    reward::RewardCollector,
    tiles::{Gem, Laser, LaserSource, Tile},
    utils::find_duplicates,
    Action, AgentId, Exit, Position, RuntimeWorldError, WorldState,
};

use super::levels;

pub struct World {
    width: usize,
    height: usize,
    world_string: String,

    grid: Vec<Vec<Rc<dyn Tile>>>,
    gems: Vec<(Position, Rc<Gem>)>,
    lasers: Vec<(Position, Rc<Laser>)>,
    sources: Vec<(Position, Rc<LaserSource>)>,

    agents: Vec<Agent>,
    start_positions: Vec<Position>,
    void_positions: Vec<Position>,
    exits: Vec<(Position, Rc<Exit>)>,
    agent_positions: Vec<Position>,
    wall_positions: Vec<Position>,

    reward_model: Rc<dyn RewardCollector>,
    available_actions: Vec<Vec<Action>>,
    done: bool,
}

impl World {
    pub fn new(
        grid: Vec<Vec<Rc<dyn Tile>>>,
        gems: Vec<(Position, Rc<Gem>)>,
        lasers: Vec<(Position, Rc<Laser>)>,
        sources: Vec<(Position, Rc<LaserSource>)>,
        start_positions: Vec<Position>,
        void_positions: Vec<Position>,
        exits: Vec<(Position, Rc<Exit>)>,
        walls_positions: Vec<Position>,
        world_str: &str,
        reward_model: Rc<dyn RewardCollector>,
    ) -> Self {
        let agents = start_positions
            .iter()
            .enumerate()
            .map(|(id, _)| Agent::new(id as u32))
            .collect();
        Self {
            width: grid[0].len(),
            height: grid.len(),
            agent_positions: start_positions.clone(),
            wall_positions: walls_positions,
            void_positions,
            agents,
            exits,
            gems,
            sources,
            grid,
            lasers,
            start_positions,
            available_actions: vec![],
            done: false,
            world_string: world_str.into(),
            reward_model,
        }
    }

    pub fn n_agents(&self) -> usize {
        self.agents.len()
    }

    pub fn world_string(&self) -> &str {
        &self.world_string
    }

    pub fn agents(&self) -> &Vec<Agent> {
        &self.agents
    }

    pub fn agents_positions(&self) -> &Vec<Position> {
        &self.agent_positions
    }

    pub fn n_gems(&self) -> usize {
        self.gems.len()
    }

    pub fn available_actions(&self) -> &Vec<Vec<Action>> {
        &self.available_actions
    }

    pub fn n_gems_collected(&self) -> usize {
        self.gems
            .iter()
            .filter(|(_, gem)| gem.is_collected())
            .count()
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

    pub fn done(&self) -> bool {
        self.done
    }

    pub fn walls(&self) -> impl Iterator<Item = &Position> {
        self.wall_positions.iter()
    }

    pub fn gems(&self) -> impl Iterator<Item = (&Position, &Gem)> {
        self.gems.iter().map(|(pos, gem)| (pos, gem.as_ref()))
    }

    pub fn laser_sources(&self) -> impl Iterator<Item = (&Position, &LaserSource)> {
        self.sources
            .iter()
            .map(|(pos, source)| (pos, source.as_ref()))
    }

    pub fn starts(&self) -> impl Iterator<Item = (AgentId, &Position)> {
        self.start_positions.iter().enumerate()
    }

    pub fn exits(&self) -> impl Iterator<Item = (&Position, &Exit)> {
        self.exits.iter().map(|(pos, exit)| (pos, exit.as_ref()))
    }

    pub fn void_positions(&self) -> impl Iterator<Item = &Position> {
        self.void_positions.iter()
    }

    pub fn lasers(&self) -> impl Iterator<Item = (&Position, &Laser)> {
        self.lasers.iter().map(|(pos, laser)| (pos, laser.as_ref()))
    }

    fn update(&mut self) {
        let mut available_actions = vec![];
        for (agent, agent_pos) in izip!(&self.agents, &self.agent_positions) {
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
        for row in self.grid.iter_mut() {
            for tile in row.iter_mut() {
                tile.reset();
            }
        }
        self.agent_positions = self.start_positions.clone();
        for ((i, j), agent) in izip!(&self.agent_positions, &self.agents) {
            self.grid[*i][*j].pre_enter(agent);
        }
        for ((i, j), agent) in izip!(&self.agent_positions, &mut self.agents) {
            self.grid[*i][*j].enter(agent);
        }
        for agent in &mut self.agents {
            agent.reset();
        }
        self.reward_model.reset();
        self.update();
    }

    /// Perform one step in the environment and return the corresponding reward.
    pub fn step(&mut self, actions: &[Action]) -> Result<f32, RuntimeWorldError> {
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
            .agent_positions
            .iter()
            .zip(actions)
            .map(|(pos, action)| {
                (action + pos).expect("Error while computing new positions: got usize underflow")
            })
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
        Ok(self.reward_model.consume())
    }

    pub fn get_state(&self) -> WorldState {
        WorldState {
            agents_positions: self.agent_positions.clone(),
            gems_collected: self
                .gems
                .iter()
                .map(|(_, gem)| gem.is_collected())
                .collect(),
        }
    }

    pub fn force_state(&mut self, state: &WorldState) -> Result<(), RuntimeWorldError> {
        if state.gems_collected.len() != self.n_gems() {
            return Err(RuntimeWorldError::InvalidNumberOfGems {
                given: state.gems_collected.len(),
                expected: self.gems.len(),
            });
        }
        if state.agents_positions.len() != self.n_agents() {
            return Err(RuntimeWorldError::InvalidNumberOfAgents {
                given: state.agents_positions.len(),
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
        // Collect the necessary gems BEFORE entering the tiles with the agents
        for ((_, gem), &collect) in izip!(&self.gems, &state.gems_collected) {
            if collect {
                gem.collect();
            }
        }
        // ONLY then, reset the reward model (after gems have been collected & before entering the tiles).
        self.reward_model.reset();
        self.agent_positions = state.agents_positions.to_vec();
        for ((i, j), agent) in izip!(&self.agent_positions, &self.agents) {
            self.grid[*i][*j].pre_enter(agent);
        }
        for ((i, j), agent) in izip!(&self.agent_positions, &mut self.agents) {
            self.grid[*i][*j].enter(agent);
        }
        // Finally, consume the reward of the "force state" step
        self.reward_model.consume();
        self.update();
        Ok(())
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
        world.force_state(&self.get_state()).unwrap();
        world
    }
}

#[cfg(test)]
#[path = "../unit_tests/test_world.rs"]
mod tests;
