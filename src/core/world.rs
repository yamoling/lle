/// The core logic of LLE, which should not be parametrisable.
use itertools::{Itertools, izip};
use std::{
    fs::File,
    io::{BufReader, Read},
};

use crate::{
    Action, AgentId, Grid, ParseError, Position, RuntimeWorldError, WorldEvent, WorldState,
    agent::{Agent, Colour},
    core::{
        boxes::{BoxId, Boxes},
        levels,
        parsing::{WorldConfig, parse},
    },
    tiles::{BoxOutcome, Button, Gem, Laser, LaserId, LaserSource, Lift, Tile},
    utils::{find_duplicates, find_duplicates_into, sample_different},
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
    boxes: Boxes,

    available_actions: Vec<Vec<Action>>,
    /// The actual start position of the agents since the last `reset`.
    start_positions: Vec<Position>,
    /// Scratch buffer reused by `solve_vertex_conflicts` across steps to avoid a fresh
    /// allocation on every call.
    conflict_scratch: Vec<bool>,
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
        box_positions: Vec<Position>,
        source_positions: Vec<Position>,
        lasers_positions: Vec<Position>,
        lift_positions: Vec<Position>,
        button_positions: Vec<Position>,
        agent_colours: Vec<Colour>,
    ) -> Self {
        let agents: Vec<Agent> = random_start_positions
            .iter()
            .enumerate()
            .map(|(id, _)| Agent::new(id, agent_colours[id]))
            .collect();
        let n_agents = agents.len();
        let (width, height, layers) = (grid.width, grid.height, grid.layers);
        let mut w = Self {
            width,
            height,
            layers,
            gems_positions: gem_positions,
            agents_positions: Vec::with_capacity(n_agents),
            random_start_positions,
            wall_positions: walls_positions,
            boxes: Boxes::new(box_positions, width, height, layers),
            void_positions,
            agents,
            exits: exit_positions,
            grid,
            start_positions: Vec::with_capacity(n_agents),
            available_actions: vec![Vec::with_capacity(5); n_agents], // There are 5 actions
            conflict_scratch: Vec::with_capacity(n_agents),
            laser_source_positions: source_positions,
            lasers_positions,
            lift_positions,
            button_positions,
            rng: rand::SeedableRng::seed_from_u64(0u64),
        };
        w.reset();
        w
    }

    pub fn n_boxes(&self) -> usize {
        self.boxes.len()
    }

    pub fn boxes_positions(&self) -> Vec<Position> {
        self.boxes.positions().clone()
    }

    /// The positions of the boxes when the world is reset.
    pub fn boxes_start_positions(&self) -> Vec<Position> {
        self.boxes.initial_positions().clone()
    }

    pub fn boxes_present(&self) -> Vec<bool> {
        self.boxes.present().clone()
    }

    /// Whether a *present* box occupies `pos`.
    pub fn has_box_at(&self, pos: Position) -> bool {
        self.boxes.id_at(pos).is_some()
    }

    /// Whether a box may be pushed onto `dest`: a walkable cell of the grid that
    /// holds neither an agent (living, dead or arrived) nor another box.
    ///
    /// A void is walkable, so pushing a box into a void is legal and destroys it.
    /// The agents' positions are checked rather than `Tile::is_occupied`, because
    /// an agent killed by a laser is not recorded on its tile.
    pub fn can_push_to(&self, dest: Position) -> bool {
        self.at(&dest).is_some_and(|tile| tile.is_walkable())
            && !self.agents_positions.contains(&dest)
            && !self.has_box_at(dest)
    }

    /// Phase 3 of the step protocol: every present box enters its current tile,
    /// turning beams off. A box that lands on a void is destroyed instead.
    /// Returns the destruction events.
    fn settle_boxes(&mut self) -> Vec<WorldEvent> {
        let mut events = vec![];
        for (id, pos) in self.boxes.present_boxes() {
            let tile = self.at_mut(&pos).expect("A box is always within the grid");
            if tile.box_enter() == BoxOutcome::Destroyed {
                self.boxes.destroy(id);
                events.push(WorldEvent::BoxDestroyed { box_id: id });
            }
        }
        events
    }

    /// Phase 2 of the step protocol: every present box leaves its tile, relighting
    /// beams, so that the reapply pass can recompute them from scratch.
    fn release_boxes(&mut self) {
        for (_, pos) in self.boxes.present_boxes() {
            self.at_mut(&pos)
                .expect("A box is always within the grid")
                .box_leave();
        }
    }

    /// Checks the box part of `state` against this world without mutating anything.
    ///
    /// Every box must lie on a walkable cell of the grid. A present box may not
    /// stand on a void, share its cell with another present box or with any agent
    /// of `state` (dead ones included). An absent box must lie on a void, since a
    /// destroyed box keeps the void cell it fell into.
    fn validate_boxes(&self, state: &WorldState) -> Result<(), RuntimeWorldError> {
        for given in [state.boxes_positions.len(), state.boxes_present.len()] {
            if given != self.n_boxes() {
                return Err(RuntimeWorldError::InvalidNumberOfBoxes {
                    given,
                    expected: self.n_boxes(),
                });
            }
        }
        let invalid = |reason: &str| RuntimeWorldError::InvalidWorldState {
            reason: reason.into(),
            state: Box::new(state.clone()),
        };
        for (pos, &present) in izip!(&state.boxes_positions, &state.boxes_present) {
            let tile = self
                .at(pos)
                .ok_or(RuntimeWorldError::OutOfWorldPosition { position: *pos })?;
            if !tile.is_walkable() {
                return Err(invalid("A box is on a non-walkable tile"));
            }
            if present && tile.is_void() {
                return Err(invalid("A present box is on a void"));
            }
            if !present && !tile.is_void() {
                return Err(invalid("An absent box is not on a void"));
            }
        }
        let present_positions = izip!(&state.boxes_positions, &state.boxes_present)
            .filter(|(_, present)| **present)
            .map(|(pos, _)| *pos)
            .collect::<Vec<_>>();
        if find_duplicates(&present_positions).iter().any(|&b| b) {
            return Err(invalid("There are two present boxes at the same position"));
        }
        if present_positions
            .iter()
            .any(|pos| state.agents_positions.contains(pos))
        {
            return Err(invalid("A present box is at the same position as an agent"));
        }
        Ok(())
    }

    pub fn n_agents(&self) -> usize {
        self.agents.len()
    }

    /// The size of the colour space: `1 + max(colour)` over agent and laser-source colours.
    /// Observation bands are indexed by colour, so this covers the largest colour value even
    /// when the colour space is sparse (e.g. agents of colours `{0, 2}` give `3`).
    pub fn n_colours(&self) -> usize {
        let max_agent_colour = self.agents.iter().map(|a| a.colour()).max();
        let max_laser_colour = self.sources().map(|(_, s)| s.colour()).max();
        match max_agent_colour.into_iter().chain(max_laser_colour).max() {
            Some(max) => max + 1,
            None => 0,
        }
    }

    /// The colour of each agent, indexed by agent id.
    pub fn agent_colours(&self) -> Vec<Colour> {
        self.agents.iter().map(|a| a.colour()).collect()
    }

    /// The number of *distinct* laser colours in the world.
    pub fn n_laser_colours(&self) -> usize {
        self.sources().map(|(_, s)| s.colour()).unique().count()
    }

    pub fn seed(&mut self, seed: u64) {
        self.rng = rand::SeedableRng::seed_from_u64(seed);
    }

    pub fn get_config(&self) -> WorldConfig {
        let source_configs = self.sources().map(|(p, s)| (p, s.into())).collect();
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
            self.boxes.initial_positions().clone(),
            source_configs,
            lift_configs,
            button_configs,
            self.agent_colours(),
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

    pub fn sources(&self) -> impl Iterator<Item = (Position, &LaserSource)> + '_ {
        self.laser_source_positions.iter().map(|&pos| {
            if let Tile::LaserSource(source) = self.grid.at(&pos) {
                (pos, source)
            } else {
                unreachable!()
            }
        })
    }

    pub fn source_at(&self, pos: Position) -> Option<&LaserSource> {
        if let Tile::LaserSource(source) = self.grid.at(&pos) {
            Some(source)
        } else {
            None
        }
    }

    pub fn lifts(&self) -> Vec<(Position, &Lift)> {
        self.lift_positions
            .iter()
            // Lifts can be wrapped into lasers.
            .map(|pos| match self.grid.at(pos).innermost() {
                Tile::Lift(lift) => (pos.clone(), lift),
                _ => unreachable!(),
            })
            .collect()
    }

    pub fn buttons(&self) -> Vec<(Position, &Button)> {
        self.button_positions
            .iter()
            // Buttons can be wrapped into lasers.
            .map(|pos| match self.grid.at(pos).innermost() {
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
                lasers.push((*pos, laser));
                if let Tile::Laser(wrapped) = laser.wrapped() {
                    lasers.push((*pos, wrapped));
                }
            } else {
                unreachable!()
            }
        }
        lasers
    }

    /// Returns an iterator over the positions of the laser beam starting from the given laser id.
    ///
    /// If the provided laser id does not exist, returns `None`.
    pub fn beam(&self, laser_id: LaserId) -> Option<impl Iterator<Item = Position>> {
        if let Some((pos, laser)) = self.sources().nth(laser_id) {
            let direction = laser.direction();
            let mut pos = (pos + direction).ok();
            Some(std::iter::from_fn(move || {
                let current = pos?;
                if let Some(Tile::Laser(_)) = self.at(&current) {
                    pos = (current + direction).ok();
                    Some(current)
                } else {
                    None
                }
            }))
        } else {
            None
        }
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
            if let Tile::Gem(gem) = self.grid.at(pos)
                && gem.is_collected()
            {
                res += 1;
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
                    boxes_positions: self.boxes.initial_positions().clone(),
                    boxes_present: vec![true; self.n_boxes()],
                },
            )
    }

    /// Recomputes `self.available_actions` in place, reusing the previous call's `Vec<Action>`
    /// allocations (each agent has at most 5 available actions) instead of dropping and
    /// reallocating a fresh `Vec<Vec<Action>>` every time.
    fn compute_available_actions(&mut self) {
        let mut buffer = std::mem::take(&mut self.available_actions);
        for (agent_actions, (agent, agent_pos)) in
            izip!(&mut buffer, izip!(&self.agents, &self.agents_positions))
        {
            agent_actions.clear();
            agent_actions.push(Action::Stay);
            if agent.is_alive() && !agent.has_arrived() {
                for action in [Action::North, Action::East, Action::South, Action::West] {
                    // Walking into a box pushes it one cell further, which is
                    // only possible if the cell beyond can hold it.
                    if let Ok(pos) = &action + agent_pos
                        && let Some(tile) = self.at(&pos)
                        && tile.is_walkable()
                        && !tile.is_occupied()
                        && (!self.has_box_at(pos)
                            || (action + pos).is_ok_and(|beyond| self.can_push_to(beyond)))
                    {
                        agent_actions.push(action);
                    }
                }
                if matches!(self.at(agent_pos), Some(tile) if tile.is_triggerable()) {
                    agent_actions.push(Action::Trigger);
                }
            }
        }
        self.available_actions = buffer;
    }

    fn solve_vertex_conflicts(&mut self, new_pos: &mut [Position]) {
        let mut conflict = true;

        while conflict {
            conflict = false;
            find_duplicates_into(new_pos, &mut self.conflict_scratch);
            for (i, is_duplicate) in self.conflict_scratch.iter().enumerate() {
                if *is_duplicate {
                    conflict = true;
                    new_pos[i] = self.agents_positions[i];
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
        // Boxes settle before the agents enter, as in `step`. No box can be
        // destroyed here: a box never starts on a void.
        self.boxes.reset();
        self.settle_boxes();
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
        self.compute_available_actions();
    }

    fn trigger_environment_actions(&mut self, actions: &[Action]) -> Vec<usize> {
        let triggers: Vec<(Colour, Position)> =
            izip!(actions, &self.agents, &self.agents_positions)
                .filter(|(action, agent, _)| **action == Action::Trigger && agent.is_alive())
                .map(|(_, agent, pos)| (agent.colour(), *pos))
                .collect();

        let mut triggered_groups = vec![];
        for (colour, pos) in triggers {
            if let Some(tile) = self.at_mut(&pos) {
                if let Some(group_id) = tile.actuate(colour) {
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
    /// out-of-bounds destination, a non-walkable destination, or a destination
    /// occupied by a box simply does nothing this tick.
    fn resolve_lift_moves(&self) -> Vec<(AgentId, Position)> {
        let mut moves = vec![];
        for (pos, lift) in self.lifts() {
            if !lift.take_triggered() {
                continue;
            }
            let Some(agent_id) = lift.agent() else {
                continue;
            };
            let rider_colour = self.agents[agent_id].colour();
            if lift
                .authorized_colour()
                .is_some_and(|auth| auth != rider_colour)
            {
                continue;
            }
            let Ok(dest) = lift.destination(pos) else {
                continue;
            };
            if matches!(self.at(&dest), Some(t) if t.is_walkable())
                && self.boxes.id_at(dest).is_none()
            {
                moves.push((agent_id, dest));
            }
        }
        moves
    }

    /// Assign `new_positions`, move the pushed boxes, run the leave/pre_enter/enter
    /// dance, and keep re-resolving while a death is still cascading.
    ///
    /// The replay passes no box moves: boxes re-settle in place, so beams are
    /// recomputed without any box moving twice.
    fn resolve_move(
        &mut self,
        new_positions: Vec<Position>,
        box_moves: &[(BoxId, Position)],
    ) -> Result<Vec<WorldEvent>, RuntimeWorldError> {
        let (mut events, mut agent_died) = self.move_agents_and_boxes(&new_positions, box_moves)?;
        self.agents_positions = new_positions.clone();
        while agent_died {
            let (additional_events, died_again) =
                self.move_agents_and_boxes(&new_positions, &[])?;
            events.extend(additional_events);
            agent_died = died_again;
        }
        Ok(events)
    }

    /// Perform one step in the environment and return the corresponding events.
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
                    taken: *action,
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
        self.solve_vertex_conflicts(&mut first_pass_positions);
        let box_moves = self.resolve_box_pushes(actions, &mut first_pass_positions);
        let mut events = self.resolve_move(first_pass_positions, &box_moves)?;

        // Trigger phase: actuate whatever tile every `Trigger`-ing agent stands on.
        let triggered_groups = self.trigger_environment_actions(actions);

        // Pass 2: lift-driven movement, only if something was actually pressed.
        // Lifts never push boxes: `resolve_lift_moves` refuses a destination
        // occupied by a box.
        if !triggered_groups.is_empty() {
            self.notify_lift_groups(&triggered_groups);
            let lift_moves = self.resolve_lift_moves();
            if !lift_moves.is_empty() {
                let mut second_pass_positions = self.agents_positions.clone();
                for (agent_id, dest) in &lift_moves {
                    second_pass_positions[*agent_id] = *dest;
                }
                self.solve_vertex_conflicts(&mut second_pass_positions);
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
                events.extend(self.resolve_move(second_pass_positions, &[])?);
            }
        }

        self.compute_available_actions();
        Ok(events)
    }

    /// The boxes pushed this step, as `(box, destination)`, given the agents'
    /// destinations after vertex conflicts. An agent pushes a box by moving onto
    /// its cell, and the box moves one cell further in the same direction.
    ///
    /// The destination of a box is claimed by its pusher. If another agent moves
    /// onto it, or another box is pushed onto it, every agent involved stays where
    /// it is (as in a vertex conflict) and the contended boxes do not move. This
    /// cannot cause new conflicts, since no agent ever moves onto an occupied cell.
    fn resolve_box_pushes(
        &self,
        actions: &[Action],
        new_positions: &mut [Position],
    ) -> Vec<(BoxId, Position)> {
        let pushes: Vec<(AgentId, BoxId, Position)> = izip!(actions, new_positions.iter())
            .enumerate()
            .filter_map(|(agent_id, (action, new))| {
                let box_id = self.boxes.id_at(*new)?;
                let dest = (action + new).expect("Checked by compute_available_actions");
                Some((agent_id, box_id, dest))
            })
            .collect();
        let mut moves = vec![];
        let mut reverted = vec![];
        for &(pusher, box_id, dest) in &pushes {
            let rival_push = pushes
                .iter()
                .any(|&(other, _, d)| other != pusher && d == dest);
            let entering = new_positions.iter().position(|pos| *pos == dest);
            if rival_push || entering.is_some() {
                reverted.push(pusher);
                reverted.extend(entering);
            } else {
                moves.push((box_id, dest));
            }
        }
        for agent_id in reverted {
            new_positions[agent_id] = self.agents_positions[agent_id];
        }
        moves
    }

    /// Moves agents and boxes through the five phases of the step protocol.
    fn move_agents_and_boxes(
        &mut self,
        new_positions: &[Position],
        box_moves: &[(BoxId, Position)],
    ) -> Result<(Vec<WorldEvent>, bool), RuntimeWorldError> {
        // Phase 1: agents leave their old tile, relighting beams.
        for (agent, pos) in izip!(&self.agents, &self.agents_positions) {
            if agent.is_alive() {
                self.grid.at_mut(pos).leave();
            }
        }
        // Phase 2: boxes leave their old tile, relighting beams. Together with
        // phase 1 this releases every blocker, so the reapply passes below can
        // recompute beam state from scratch — `LaserBeam::turn_on` fills
        // `[offset..]` unconditionally, so a partial release would be wrong.
        self.release_boxes();
        // Phase 3: boxes move, then settle on their new tile, turning beams off.
        // This must precede phase 5 (phase 4 commutes with it, as both only turn
        // beams off): an agent pushing a box toward a laser source is saved by the
        // box it just pushed.
        for (box_id, dest) in box_moves {
            self.boxes.set_position(*box_id, *dest);
        }
        let mut events = self.settle_boxes();
        // Phase 4: agents pre-enter, turning off beams of their own colour.
        for (agent, pos) in izip!(&self.agents, new_positions) {
            self.grid
                .at_mut(pos)
                .pre_enter(agent)
                .expect("When moving agents, the pre-enter should not fail");
        }
        // Phase 5: agents enter; deaths resolve here.
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
            boxes_positions: self.boxes.positions().clone(),
            boxes_present: self.boxes.present().clone(),
        }
    }

    /// Restore the world's agents and gems to an explicitly supplied state.
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
                state: Box::new(state.clone()),
            });
        }

        for pos in &state.agents_positions {
            if pos.i >= self.height || pos.j >= self.width || pos.k >= self.layers {
                return Err(RuntimeWorldError::OutOfWorldPosition { position: *pos });
            }
        }
        self.validate_boxes(state)?;
        let current_state = self.get_state();

        // Reset tiles and agents (but do not enter the new tiles)
        for (_, tile) in self.grid.iter_mut() {
            tile.reset();
        }
        // Collect the necessary gems BEFORE entering the tiles with the agents
        for (pos, &collect) in izip!(&self.gems_positions, &state.gems_collected) {
            if collect {
                match self.grid.at_mut(pos) {
                    Tile::Gem(gem) => gem.collect(),
                    Tile::Laser(laser) => laser
                        .gem_mut()
                        .expect("Every gem position should contain a gem")
                        .collect(),
                    _ => unreachable!("Every gem position should contain a gem"),
                }
            }
        }
        // Boxes settle before the agents enter, as in `step`. `validate_boxes`
        // guarantees that no present box is on a void, so nothing is destroyed here.
        self.boxes
            .restore(&state.boxes_positions, &state.boxes_present);
        self.settle_boxes();
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
                    position: *pos,
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
                state: Box::new(state.clone()),
            });
        }
        self.compute_available_actions();
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
        let mut clone = self.get_config().into_world().unwrap();
        clone.set_state(&state).unwrap();
        clone
    }
}

#[cfg(test)]
#[path = "../unit_tests/test_world.rs"]
mod test;

#[cfg(test)]
#[path = "../unit_tests/test_agent_colour.rs"]
mod test_agent_colour;
