/// The core logic of LLE, which should not be parametrisable.
use itertools::{Itertools, izip};
use std::{
    fs::File,
    io::{BufReader, Read},
};

use crate::{
    Action, AgentId, Grid, ParseError, Position, RuntimeWorldError, WorldEvent, WorldState,
    agent::Agent,
    core::{
        levels,
        parsing::{WorldConfig, parse},
    },
    tiles::{Button, Gem, Laser, LaserSource, Lift, Tile},
    utils::{find_duplicates, sample_different},
};

type JointAction = Vec<Action>;

pub struct World {
    width: usize,
    height: usize,
    layers: usize,

    grid: Grid<Tile>,
    agents: Vec<Agent>,
    laser_source_positions: Vec<Position>,
    lasers_positions: Vec<Position>,
    gems_positions: Vec<Position>,
    lift_positions: Vec<Position>,
    button_positions: Vec<Position>,
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
        grid: Grid<Tile>,
        gem_positions: Vec<Position>,
        random_start_positions: Vec<Vec<Position>>,
        void_positions: Vec<Position>,
        exit_positions: Vec<Position>,
        walls_positions: Vec<Position>,
        source_positions: Vec<Position>,
        lasers_positions: Vec<Position>,
        lift_positions: Vec<Position>,
        button_positions: Vec<Position>,
    ) -> Self {
        let agents = random_start_positions
            .iter()
            .enumerate()
            .map(|(id, _)| Agent::new(id as AgentId))
            .collect();
        let mut w = Self {
            width: grid.width,
            height: grid.height,
            layers: grid.layers,
            gems_positions: gem_positions,
            agents_positions: vec![],
            random_start_positions,
            wall_positions: walls_positions,
            void_positions,
            agents,
            exits: exit_positions,
            grid,
            start_positions: vec![],
            available_actions: vec![],
            laser_source_positions: source_positions,
            lasers_positions,
            lift_positions,
            button_positions,
            rng: rand::SeedableRng::seed_from_u64(0u64),
        };
        w.reset();
        w
    }

    pub fn n_agents(&self) -> usize {
        self.agents.len()
    }

    pub fn n_laser_colours(&self) -> usize {
        self.sources()
            .iter()
            .map(|(_, s)| s.agent_id())
            .unique()
            .count()
    }

    pub fn seed(&mut self, seed: u64) {
        self.rng = rand::SeedableRng::seed_from_u64(seed);
    }

    pub fn get_config(&self) -> WorldConfig {
        let source_configs = self
            .sources()
            .into_iter()
            .map(|(p, s)| (p, s.into()))
            .collect();
        let lift_configs = self
            .lifts()
            .into_iter()
            .map(|(pos, lift)| (pos, lift.into()))
            .collect();
        let button_configs = self
            .buttons()
            .into_iter()
            .map(|(pos, button)| (pos, button.into()))
            .collect();
        WorldConfig::new(
            self.width,
            self.height,
            self.layers,
            self.gems_positions.clone(),
            self.random_start_positions.clone(),
            self.void_positions.clone(),
            self.exits.clone(),
            self.wall_positions.clone(),
            source_configs,
            lift_configs,
            button_configs,
        )
    }

    /// The world string, taking into account the fact that some tiles may have changed (laser direction or colour).
    pub fn world_string(&self) -> String {
        self.get_config().to_string()
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
            .map(|pos| match self.grid.at(pos) {
                Tile::Gem(gem) => gem,
                Tile::Laser(laser) => laser.gem().unwrap(),
                _ => unreachable!(),
            })
            .collect()
    }

    pub fn sources(&self) -> Vec<(Position, &LaserSource)> {
        self.laser_source_positions
            .iter()
            .map(|pos| match self.grid.at(pos) {
                Tile::LaserSource(source) => (pos.clone(), source),
                _ => unreachable!(),
            })
            .collect()
    }

    pub fn lifts(&self) -> Vec<(Position, &Lift)> {
        self.lift_positions
            .iter()
            .map(|pos| match self.grid.at(pos) {
                Tile::Lift(lift) => (pos.clone(), lift),
                _ => unreachable!(),
            })
            .collect()
    }

    pub fn buttons(&self) -> Vec<(Position, &Button)> {
        self.button_positions
            .iter()
            .map(|pos| match self.grid.at(pos) {
                Tile::Button(button) => (pos.clone(), button),
                _ => unreachable!(),
            })
            .collect()
    }

    pub fn n_lifts(&self) -> usize {
        self.lift_positions.len()
    }

    pub fn n_buttons(&self) -> usize {
        self.button_positions.len()
    }

    pub fn lasers(&self) -> Vec<(Position, &Laser)> {
        let mut lasers = vec![];
        for pos in &self.lasers_positions {
            if let Tile::Laser(laser) = self.grid.at(pos) {
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

    pub fn set_exit_positions(&mut self, exits: Vec<Position>) -> Result<(), ParseError> {
        if exits.len() < self.n_agents() {
            return Err(ParseError::NotEnoughExitTiles {
                n_starts: self.n_agents(),
                n_exits: exits.len(),
            });
        }
        // Replace current exits by floor tiles
        for pos in &self.exits {
            let tile = self.grid.pop(pos);
            let replacement = match tile {
                Tile::Exit { agent } => Tile::Floor { agent },
                Tile::Laser(mut laser) => {
                    laser.set_tile(Tile::Floor {
                        agent: laser.agent(),
                    });
                    Tile::Laser(laser)
                }
                other => panic!("Tile is not an exit: {:?}", other),
            };
            self.grid.insert(pos, replacement);
        }
        // Set new exits
        self.exits = exits;
        for pos in &self.exits {
            let tile = self.grid.pop(pos);
            let replacement = match tile {
                Tile::Floor { agent } => Tile::Exit { agent },
                Tile::Laser(mut laser) => {
                    laser.set_tile(Tile::Exit {
                        agent: laser.agent(),
                    });
                    Tile::Laser(laser)
                }
                other => panic!("Tile is not a floor: {:?}", other),
            };
            self.grid.insert(pos, replacement);
        }
        Ok(())
    }

    pub fn exits_positions(&self) -> Vec<Position> {
        self.exits.clone()
    }

    pub fn n_exits(&self) -> usize {
        self.exits.len()
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
            if let Tile::Gem(gem) = self.grid.at(pos) {
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

    pub fn layers(&self) -> usize {
        self.layers
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

    /// Iterator over all the possible states of the World.
    /// NOT YET TESTED
    pub fn all_states(
        &'_ self,
        restrict_to_alive_agents: bool,
    ) -> impl Iterator<Item = WorldState> + '_ {
        let agents_positions = self
            .get_state_space()
            .into_iter()
            .map(|(i, j, k)| Position { i, j, k })
            .filter(|pos| !self.wall_positions.contains(pos))
            .combinations(self.n_agents()); //TODO: need to add layers support

        let collection_status = (0..self.n_gems())
            .map(|_| vec![true, false])
            .multi_cartesian_product();

        let alive_status = match restrict_to_alive_agents {
            true => vec![true],
            false => vec![true, false],
        }
        .into_iter()
        .combinations(self.n_agents());

        agents_positions
            .cartesian_product(collection_status)
            .cartesian_product(alive_status)
            .map(
                |((agents_positions, gems_collected), agents_alive)| WorldState {
                    agents_positions,
                    gems_collected,
                    agents_alive,
                },
            )
    }

    fn compute_available_actions(&self) -> Vec<Vec<Action>> {
        let mut available_actions = vec![];
        for (agent, agent_pos) in izip!(&self.agents, &self.agents_positions) {
            let mut agent_actions = vec![Action::Stay];
            if agent.is_alive() && !agent.has_arrived() {
                for action in [Action::North, Action::East, Action::South, Action::West] {
                    if let Ok(pos) = &action + agent_pos {
                        if let Some(tile) = self.at(&pos) {
                            if tile.is_walkable() && !tile.is_occupied() {
                                agent_actions.push(action);
                            }
                        }
                    }
                }
                if matches!(self.at(agent_pos), Some(tile) if tile.is_triggerable()) {
                    agent_actions.push(Action::Trigger);
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
    /// tiles work was transferred into Grid in multi-floor branch
    pub fn tiles(&self) -> Vec<(Position, &Tile)> {
        let mut res = vec![];
        res.extend(self.grid.iter().map(|(pos, tile)| (pos.clone(), tile))); //? .clone is maybe not useful need review
        res
    }

    pub fn at(&self, pos: &Position) -> Option<&Tile> {
        if pos.i >= self.height {
            return None;
        }
        if pos.j >= self.width {
            return None;
        }
        if pos.k >= self.layers {
            return None;
        }
        Some(self.grid.at(pos))
    }

    pub fn at_mut(&mut self, pos: &Position) -> Option<&mut Tile> {
        if pos.i >= self.height {
            return None;
        }
        if pos.j >= self.width {
            return None;
        }
        if pos.k >= self.layers {
            return None;
        }
        Some(self.grid.at_mut(pos))
    }

    pub fn reset(&mut self) {
        for (_, tile) in self.grid.iter_mut() {
            tile.reset();
        }
        // Reset (dead=false) the agents such that they can block lasers on spacwn
        for agent in &mut self.agents {
            agent.reset();
        }
        self.start_positions = sample_different(&mut self.rng, &self.random_start_positions);
        self.agents_positions = self.start_positions.clone();
        for (pos, agent) in izip!(&self.agents_positions, &self.agents) {
            self.grid
                .at_mut(pos)
                .pre_enter(agent)
                .expect("The agent should be able to pre-enter");
        }
        for (pos, agent) in izip!(&self.agents_positions, &mut self.agents) {
            self.grid.at_mut(pos).enter(agent);
        }
        self.available_actions = self.compute_available_actions();
    }

    fn trigger_environment_actions(&mut self, actions: &[Action]) -> Vec<usize> {
        let trigger_positions: Vec<Position> = izip!(actions, &self.agents, &self.agents_positions)
            .filter(|(action, agent, _)| **action == Action::Trigger && agent.is_alive())
            .map(|(_, _, pos)| *pos)
            .collect();

        let mut triggered_groups = vec![];
        for pos in trigger_positions {
            if let Some(tile) = self.at_mut(&pos) {
                if let Some(group_id) = tile.actuate() {
                    if !triggered_groups.contains(&group_id) {
                        triggered_groups.push(group_id);
                    }
                }
            }
        }
        triggered_groups
    }

    /// Pulse every `Lift` sharing one of `group_ids`.
    fn notify_lift_groups(&self, group_ids: &[usize]) {
        for (_, lift) in self.lifts() {
            if group_ids.contains(&lift.group_id()) {
                lift.notify();
            }
        }
    }

    /// Consume every lift's pulse flag and compute the (agent, destination)
    /// relocation it causes, if any. A lift with no occupant, an
    /// out-of-bounds destination, or a non-walkable destination simply does
    /// nothing this tick.
    fn resolve_lift_moves(&self) -> Vec<(AgentId, Position)> {
        let mut moves = vec![];
        for (pos, lift) in self.lifts() {
            if !lift.take_triggered() {
                continue;
            }
            let Some(agent_id) = lift.agent() else {
                continue;
            };
            if lift
                .authorized_agent_id()
                .is_some_and(|auth| auth != agent_id)
            {
                continue;
            }
            let Ok(dest) = lift.destination(pos) else {
                continue;
            };
            if matches!(self.at(&dest), Some(t) if t.is_walkable()) {
                moves.push((agent_id, dest));
            }
        }
        moves
    }

    /// Assign `new_positions`, run the leave/pre_enter/enter dance, and keep
    /// re-resolving while a death is still cascading.
    fn resolve_move(
        &mut self,
        new_positions: Vec<Position>,
    ) -> Result<Vec<WorldEvent>, RuntimeWorldError> {
        let (mut events, mut agent_died) = self.move_agents(&new_positions)?;
        self.agents_positions = new_positions.clone();
        while agent_died {
            let (additional_events, died_again) = self.move_agents(&new_positions)?;
            events.extend(additional_events);
            agent_died = died_again;
        }
        Ok(events)
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

        // Positions are computed in two passes:
        // - first: the natural movement of the agents according to their actions
        // - second: environment-triggered movement (lifts pulsed by a button press)
        let mut first_pass_positions = self
            .agents_positions
            .iter()
            .zip(actions)
            .map(|(pos, action)| action + pos)
            .collect::<Result<Vec<Position>, RuntimeWorldError>>()?;

        // Check for vertex conflicts
        // If a new_pos occurs more than once, then set it back to its original position
        World::solve_vertex_conflicts(&mut first_pass_positions, &self.agents_positions);
        let mut events = self.resolve_move(first_pass_positions)?;

        // Trigger phase: actuate whatever tile every `Trigger`-ing agent stands on.
        let triggered_groups = self.trigger_environment_actions(actions);

        // Pass 2: lift-driven movement, only if something was actually pressed.
        if !triggered_groups.is_empty() {
            self.notify_lift_groups(&triggered_groups);
            let lift_moves = self.resolve_lift_moves();
            if !lift_moves.is_empty() {
                let mut second_pass_positions = self.agents_positions.clone();
                for (agent_id, dest) in &lift_moves {
                    second_pass_positions[*agent_id] = *dest;
                }
                World::solve_vertex_conflicts(&mut second_pass_positions, &self.agents_positions);
                // Only emit an event for agents whose lift-relocation actually
                // survived vertex-conflict resolution (a colliding destination
                // reverts the agent back to its lift, in which case there was no
                // move to report).
                for (agent_id, dest) in &lift_moves {
                    if second_pass_positions[*agent_id] == *dest {
                        events.push(WorldEvent::LiftMoved {
                            agent_id: *agent_id,
                            from: self.agents_positions[*agent_id],
                            to: *dest,
                        });
                    }
                }
                events.extend(self.resolve_move(second_pass_positions)?);
            }
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
                self.grid.at_mut(pos).leave();
            }
        }
        // Pre-enter
        for (agent, pos) in izip!(&self.agents, new_positions) {
            self.grid
                .at_mut(pos)
                .pre_enter(agent)
                .expect("When moving agents, the pre-enter should not fail");
        }
        // Enter
        let mut events = vec![];
        let mut agent_died = false;
        for (agent, pos) in izip!(&mut self.agents, new_positions) {
            if let Some(event) = self.grid.at_mut(pos).enter(agent) {
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
            if pos.i >= self.height || pos.j >= self.width || pos.k >= self.layers {
                return Err(RuntimeWorldError::OutOfWorldPosition {
                    position: pos.clone(),
                });
            }
        }
        let current_state = self.get_state();

        // Reset tiles and agents (but do not enter the new tiles)
        for (_, tile) in self.grid.iter_mut() {
            tile.reset();
        }
        // Collect the necessary gems BEFORE entering the tiles with the agents
        for (pos, &collect) in izip!(&self.gems_positions, &state.gems_collected) {
            if collect {
                if let Tile::Gem(gem) = &mut self.grid.at_mut(pos) {
                    gem.collect();
                }
            }
        }
        for (pos, agent) in izip!(&state.agents_positions, &self.agents) {
            if let Err(error) = self.grid.at_mut(pos).pre_enter(agent) {
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
        self.agents_positions = state.agents_positions.clone();
        let mut events = vec![];
        for (pos, alive, agent) in izip!(
            &self.agents_positions,
            &state.agents_alive,
            &mut self.agents
        ) {
            agent.reset();
            if let Some(event) = self.grid.at_mut(pos).enter(agent) {
                events.push(event);
            }
            // If agents were specifically set to be dead, then do so.
            if !alive {
                agent.die();
            }
        }

        let actual_state = self.get_state();
        if actual_state != *state {
            return Err(RuntimeWorldError::InvalidWorldState {
                reason: "The given state is invalid (e.g. an agent whose alive status was set to `true` died).".into(),
                state: state.clone(),
            });
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
                });
            }
        };
        let mut reader = BufReader::new(file);
        let mut world_str = String::new();
        reader.read_to_string(&mut world_str).unwrap();
        World::try_from(world_str)
    }

    fn get_state_space(&self) -> Vec<(usize, usize, usize)> {
        vec![(0..self.height), (0..self.width), (0..self.layers)]
            .into_iter()
            .multi_cartesian_product()
            .map(|v| (v[0], v[1], v[2]))
            .collect_vec() // Added overhead but more readable for future modifications
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
        let state = self.get_state();
        let mut clone = self.get_config().to_world().unwrap();
        clone.set_state(&state).unwrap();
        clone
    }
}

#[cfg(test)]
mod test;
