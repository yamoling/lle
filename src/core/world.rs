/// The core logic of LLE, which should not be parametrisable.
use itertools::{Itertools, izip};
use std::{
    fs::File,
    io::{BufReader, Read},
};

use crate::{
    Action, ParseError, Position, RuntimeWorldError, WorldEvent, WorldState,
    agent::{Agent, AgentId, Colour},
    core::{
        boxes::{BoxId, Boxes},
        levels,
        parsing::{WorldConfig, parse},
    },
    tiles::{BoxOutcome, Gem, Laser, LaserId, LaserSource, Tile},
    utils::{find_duplicates, find_duplicates_into, sample_different},
};

type JointAction = Vec<Action>;

pub struct World {
    width: usize,
    height: usize,

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
        grid: Vec<Vec<Tile>>,
        gem_positions: Vec<Position>,
        random_start_positions: Vec<Vec<Position>>,
        void_positions: Vec<Position>,
        exit_positions: Vec<Position>,
        walls_positions: Vec<Position>,
        box_positions: Vec<Position>,
        source_positions: Vec<Position>,
        lasers_positions: Vec<Position>,
        agent_colours: Vec<Colour>,
    ) -> Self {
        let agents: Vec<Agent> = random_start_positions
            .iter()
            .enumerate()
            .map(|(id, _)| Agent::new(id, agent_colours[id]))
            .collect();
        let n_agents = agents.len();
        let (width, height) = (grid[0].len(), grid.len());
        let mut w = Self {
            width,
            height,
            gems_positions: gem_positions,
            agents_positions: Vec::with_capacity(n_agents),
            random_start_positions,
            wall_positions: walls_positions,
            boxes: Boxes::new(box_positions, width, height),
            void_positions,
            agents,
            exits: exit_positions,
            grid,
            start_positions: Vec::with_capacity(n_agents),
            available_actions: vec![Vec::with_capacity(5); n_agents], // There are 5 actions
            conflict_scratch: Vec::with_capacity(n_agents),
            laser_source_positions: source_positions,
            lasers_positions,
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

    pub fn boxes_present(&self) -> Vec<bool> {
        self.boxes.present().clone()
    }

    /// Whether a *present* box occupies `pos`.
    pub fn has_box_at(&self, pos: Position) -> bool {
        self.boxes.id_at(pos).is_some()
    }

    pub fn box_id_at(&self, pos: Position) -> Option<BoxId> {
        self.boxes.id_at(pos)
    }

    /// Whether a box may come to rest on `dest`. Pure: no side effects.
    ///
    /// A void is walkable, so pushing a box into a void is legal and destroys it.
    ///
    /// A cell holding any agent — living, dead or arrived — is refused, since
    /// none of them vacate their cell. Tile occupancy alone does not cover that:
    /// an agent killed by a beam is never recorded by the laser's wrapped tile,
    /// so the agents' positions are checked as well.
    pub fn can_push_to(&self, dest: Position) -> bool {
        match self.at(&dest) {
            Some(tile) => {
                tile.is_walkable()
                    && !tile.is_occupied()
                    && !self.agents_positions.contains(&dest)
                    && !self.has_box_at(dest)
            }
            None => false,
        }
    }

    /// Phase 3 of the step protocol: every present box enters its current tile,
    /// turning beams off. A box that lands on a void is destroyed instead.
    /// Returns the destruction events.
    fn settle_boxes(&mut self) -> Vec<WorldEvent> {
        let mut events = vec![];
        for id in 0..self.boxes.len() {
            if !self.boxes.present()[id] {
                continue;
            }
            let pos = self.boxes.positions()[id];
            let outcome = self
                .at_mut(&pos)
                .expect("A box is always within the grid")
                .box_enter();
            if outcome == BoxOutcome::Destroyed {
                self.boxes.destroy(id);
                events.push(WorldEvent::BoxDestroyed { box_id: id });
            }
        }
        events
    }

    /// Phase 2 of the step protocol: every present box leaves its tile, relighting
    /// beams, so that the reapply pass can recompute them from scratch.
    fn release_boxes(&mut self) {
        for id in 0..self.boxes.len() {
            if !self.boxes.present()[id] {
                continue;
            }
            let pos = self.boxes.positions()[id];
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
        if state.boxes_positions.len() != self.n_boxes()
            || state.boxes_present.len() != self.n_boxes()
        {
            let given = if state.boxes_positions.len() != self.n_boxes() {
                state.boxes_positions.len()
            } else {
                state.boxes_present.len()
            };
            return Err(RuntimeWorldError::InvalidNumberOfBoxes {
                given,
                expected: self.n_boxes(),
            });
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
        WorldConfig::new(
            self.width,
            self.height,
            self.gems_positions.clone(),
            self.random_start_positions.clone(),
            self.void_positions.clone(),
            self.exits.clone(),
            self.wall_positions.clone(),
            self.boxes.initial_positions().clone(),
            source_configs,
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
            .map(|pos| match &self.grid[pos.i][pos.j] {
                Tile::Gem(gem) => gem,
                Tile::Laser(laser) => laser.gem().unwrap(),
                _ => unreachable!(),
            })
            .collect()
    }

    pub fn sources(&self) -> impl Iterator<Item = (Position, &LaserSource)> + '_ {
        self.laser_source_positions.iter().map(|&pos| {
            if let Tile::LaserSource(source) = &self.grid[pos.i][pos.j] {
                (pos, source)
            } else {
                unreachable!()
            }
        })
    }

    pub fn source_at(&self, pos: Position) -> Option<&LaserSource> {
        if let Tile::LaserSource(source) = &self.grid[pos.i][pos.j] {
            Some(source)
        } else {
            None
        }
    }

    pub fn lasers(&self) -> Vec<(Position, &Laser)> {
        let mut lasers = vec![];
        for pos in &self.lasers_positions {
            if let Tile::Laser(laser) = &self.grid[pos.i][pos.j] {
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
            let tile = self.grid[pos.i].remove(pos.j);
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
            self.grid[pos.i].insert(pos.j, replacement);
        }
        // Set new exits
        self.exits = exits;
        for pos in &self.exits {
            let tile = self.grid[pos.i].remove(pos.j);
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
            self.grid[pos.i].insert(pos.j, replacement);
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
            if let Tile::Gem(gem) = &self.grid[pos.i][pos.j]
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
        let agents_positions = (0..self.height)
            .cartesian_product(0..self.width)
            .map(|(i, j)| Position { i, j })
            .filter(|pos| !self.wall_positions.contains(pos))
            .combinations(self.n_agents());

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
                    let Ok(dest) = &action + agent_pos else {
                        continue;
                    };
                    let Some(tile) = self.at(&dest) else {
                        continue;
                    };
                    if !tile.is_walkable() || tile.is_occupied() {
                        continue;
                    }
                    if self.has_box_at(dest) {
                        // Walking into a box pushes it one further; legal only if
                        // the cell beyond can hold it.
                        let Ok(beyond) = action + dest else {
                            continue;
                        };
                        if !self.can_push_to(beyond) {
                            continue;
                        }
                    }
                    agent_actions.push(action);
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
        // Boxes settle before the agents pre-enter (spec phase order). Destruction
        // events cannot occur on reset: a box never starts on a void.
        self.boxes.reset();
        self.settle_boxes();
        // Reset (dead=false) the agents such that they can block lasers on spacwn
        for agent in &mut self.agents {
            agent.reset();
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
        self.compute_available_actions();
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
        let mut new_positions = self
            .agents_positions
            .iter()
            .zip(actions)
            .map(|(pos, action)| action + pos)
            .collect::<Result<Vec<Position>, RuntimeWorldError>>()?;

        // Check for vertex conflicts
        // If a new_pos occurs more than once, then set it back to its original position
        self.solve_vertex_conflicts(&mut new_positions);
        // Pushes may revert more agents (contended box destinations).
        let box_moves = self.resolve_box_pushes(&mut new_positions);
        let (mut events, mut agent_died) =
            self.move_agents_and_boxes(&new_positions, &box_moves)?;
        self.agents_positions.clone_from(&new_positions);
        // At this stage, all agents are on their new positions.
        // However, some events (death) could still happen if an agent has died.
        // The replay passes no box moves: boxes re-settle in place, so beams are
        // recomputed without any box moving twice.
        while agent_died {
            let (additional_events, died2) = self.move_agents_and_boxes(&new_positions, &[])?;
            events.extend(additional_events);
            agent_died = died2;
        }
        self.compute_available_actions();
        Ok(events)
    }

    /// The pushes implied by the agents' destinations, as `(pusher, box, box destination)`.
    ///
    /// An agent pushes a box when it moves onto the box's cell; the box moves one
    /// cell further in the same direction. Pure: it neither checks nor resolves
    /// contention, see `resolve_box_pushes`.
    fn box_pushes(&self, new_agent_positions: &[Position]) -> Vec<(AgentId, BoxId, Position)> {
        let mut pushes: Vec<(AgentId, BoxId, Position)> = vec![];
        for (agent_id, (old, new)) in izip!(&self.agents_positions, new_agent_positions).enumerate()
        {
            if old == new {
                continue;
            }
            let Some(box_id) = self.boxes.id_at(*new) else {
                continue;
            };
            let delta = (new.i as i32 - old.i as i32, new.j as i32 - old.j as i32);
            let action = Action::try_from(delta).expect("An agent moves by one cell at a time");
            let dest =
                (&action + new).expect("Push legality was checked in compute_available_actions");
            debug_assert!(
                !pushes.iter().any(|(_, id, _)| *id == box_id),
                "at most one agent may push a given box per step"
            );
            pushes.push((agent_id, box_id, dest));
        }
        pushes
    }

    /// Which boxes move this step, given the agents' post-conflict destinations.
    /// Agents whose pushes contend are reverted in `new_agent_positions`.
    ///
    /// Runs *after* `solve_vertex_conflicts`, which buys two properties for free:
    /// an agent reverted by a conflict does not move its box, and two agents
    /// pushing one box from different axes both want the box's cell, so both
    /// revert. Head-on pushes cannot arise — each pusher's destination is the
    /// other's current cell, so neither action is ever available.
    ///
    /// A box's destination is a cell claimed by its pusher. When it is also
    /// another agent's new position, or another pushed box's destination, every
    /// agent involved reverts to its current position — the same "all parties
    /// revert" rule as a vertex conflict. Vertex conflicts and pushes are then
    /// re-resolved until a fixed point. Today a revert cannot cascade (agents
    /// never walk into an occupied cell, so nobody follows a reverted agent, and
    /// a reverted pusher's box stays on a cell no other push may target), so the
    /// loop runs at most twice; it is kept so that a future rule allowing
    /// following stays correct. It terminates: every unresolved pass reverts at
    /// least one moving pusher, and a reverted agent never moves again.
    ///
    /// Takes `&mut self` only for the scratch buffer of `solve_vertex_conflicts`.
    fn resolve_box_pushes(
        &mut self,
        new_agent_positions: &mut [Position],
    ) -> Vec<(BoxId, Position)> {
        loop {
            let pushes = self.box_pushes(new_agent_positions);
            if pushes.is_empty() {
                return Vec::new();
            }
            let mut revert = vec![false; new_agent_positions.len()];
            for (k, (pusher, _, dest)) in pushes.iter().enumerate() {
                let rival_box = pushes
                    .iter()
                    .enumerate()
                    .any(|(l, (_, _, other))| l != k && other == dest);
                // After vertex conflicts, at most one agent can be heading to `dest`.
                let entering = new_agent_positions.iter().position(|p| p == dest);
                if rival_box || entering.is_some() {
                    revert[*pusher] = true;
                    if let Some(agent_id) = entering {
                        revert[agent_id] = true;
                    }
                }
            }
            if !revert.contains(&true) {
                return pushes
                    .into_iter()
                    .map(|(_, box_id, dest)| (box_id, dest))
                    .collect();
            }
            for (new, old, reverted) in izip!(
                new_agent_positions.iter_mut(),
                &self.agents_positions,
                revert
            ) {
                if reverted {
                    *new = *old;
                }
            }
            self.solve_vertex_conflicts(new_agent_positions);
        }
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
                self.grid[pos.i][pos.j].leave();
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
            self.grid[pos.i][pos.j]
                .pre_enter(agent)
                .expect("When moving agents, the pre-enter should not fail");
        }
        // Phase 5: agents enter; deaths resolve here.
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
            if pos.i >= self.height || pos.j >= self.width {
                return Err(RuntimeWorldError::OutOfWorldPosition { position: *pos });
            }
        }
        self.validate_boxes(state)?;
        let current_state = self.get_state();

        // Reset tiles and agents (but do not enter the new tiles)
        for row in &mut self.grid {
            for tile in row {
                tile.reset();
            }
        }
        // Collect the necessary gems BEFORE entering the tiles with the agents
        for (pos, &collect) in izip!(&self.gems_positions, &state.gems_collected) {
            if collect {
                match &mut self.grid[pos.i][pos.j] {
                    Tile::Gem(gem) => gem.collect(),
                    Tile::Laser(laser) => laser
                        .gem_mut()
                        .expect("Every gem position should contain a gem")
                        .collect(),
                    _ => unreachable!("Every gem position should contain a gem"),
                }
            }
        }
        // Boxes settle before the agents pre-enter (spec phase order). The restored
        // state already records which boxes are absent, so no destruction event
        // belongs to a `set_state`: the settle events are discarded.
        self.boxes
            .restore(&state.boxes_positions, &state.boxes_present);
        self.settle_boxes();
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
            if let Some(event) = self.grid[pos.i][pos.j].enter(agent) {
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
