use std::collections::{HashMap, HashSet, VecDeque};

use strum::IntoEnumIterator;

use crate::Position;
use crate::{World, tiles::Direction};

fn neighbours_of(
    pos: Position,
    exits: &HashSet<Position>,
    height: usize,
    width: usize,
    walls: &HashSet<Position>,
) -> Vec<Position> {
    if exits.contains(&pos) {
        // Once an agent reaches an exit, it can no longer move.
        return vec![pos];
    }
    let mut result = Vec::new();
    for d in Direction::iter() {
        if let Ok(n) = pos + d {
            if n.i < height && n.j < width && !walls.contains(&n) {
                result.push(n);
            }
        }
    }
    result
}

/// One laser source's relevant info for constraint generation.
pub struct LaserSourceInfo {
    pub agent_id: usize,
    pub laser_id: usize,
    /// Beam tiles, in order, starting right after the source tile.
    pub path: Vec<Position>,
}

/// Data shared across all constraint-generation routines, mirroring the Python `ConstraintContext`.
/// Built once per `(world, t_max)` pair. Expensive data like time-wise reachability is computed
/// on-demand and cached for efficiency.
pub struct ConstraintContext {
    pub t_max: usize,
    pub n_agents: usize,
    pub start_pos: Vec<Position>,
    pub exits: HashSet<Position>,
    pub predecessors: HashMap<Position, Vec<Position>>,
    pub solution_lower_bound: usize,
    pub laser_sources: Vec<LaserSourceInfo>,

    // Neighborhood and distance information (computed once at construction).
    neighbours: HashMap<Position, Vec<Position>>,
    exit_distance: HashMap<Position, usize>,

    // Cached on-demand data (lazily computed per time step).
    /// Cache for `exit_reachable[t]`: positions from which an exit can still be reached
    /// within `t_max - t` steps.
    exit_reachable_cache: HashMap<usize, HashSet<Position>>,

    /// Cache for reachable positions per agent and time step.
    reachable_positions_cache: HashMap<(usize, usize), HashSet<Position>>,

    /// Cache for reachable laser paths per laser source and time step.
    reachable_laser_paths_cache: HashMap<(usize, usize), Vec<Position>>,
}

impl ConstraintContext {
    pub fn new(world: &World, t_max: usize) -> Self {
        let height = world.height();
        let width = world.width();
        let n_agents = world.n_agents();
        let walls: HashSet<Position> = world.walls().into_iter().collect();
        let voids: HashSet<Position> = world.void_positions().into_iter().collect();
        let exits: HashSet<Position> = world.exits_positions().into_iter().collect();
        let start_pos: Vec<Position> = world.starts().into_iter().collect();

        let mut valid_positions = HashSet::new();
        for i in 0..height {
            for j in 0..width {
                let pos = Position { i, j };
                if !walls.contains(&pos) && !voids.contains(&pos) {
                    valid_positions.insert(pos);
                }
            }
        }

        // neighbours[pos] = [pos, ...reachable single-step neighbours]
        let mut neighbours: HashMap<Position, Vec<Position>> = HashMap::new();
        for &pos in &valid_positions {
            let mut succ = vec![pos];
            for n in neighbours_of(pos, &exits, height, width, &walls) {
                if valid_positions.contains(&n) {
                    succ.push(n);
                }
            }
            neighbours.insert(pos, succ);
        }

        // Reverse adjacency: predecessors[p] = positions from which an agent can move into p.
        let mut predecessors: HashMap<Position, Vec<Position>> =
            valid_positions.iter().map(|&p| (p, Vec::new())).collect();
        for (&pos, successors) in &neighbours {
            for &succ in successors {
                predecessors.get_mut(&succ).unwrap().push(pos);
            }
        }

        let exit_distance = compute_exit_distance(&exits, &predecessors);

        let solution_lower_bound = start_pos
            .iter()
            .map(|p| exit_distance.get(p).copied().unwrap_or(0))
            .max()
            .unwrap_or(0);

        let mut laser_sources = Vec::new();
        for (pos, source) in world.sources() {
            let d = source.direction();
            let mut prev = pos;
            let mut path = Vec::new();
            while let Ok(current) = prev + d
                && current.i < height
                && current.j < width
                && !walls.contains(&current)
            {
                path.push(current);
                prev = current;
            }
            laser_sources.push(LaserSourceInfo {
                agent_id: source.agent_id(),
                laser_id: source.laser_id(),
                path,
            });
        }

        ConstraintContext {
            t_max,
            n_agents,
            start_pos,
            exits,
            predecessors,
            solution_lower_bound,
            laser_sources,
            neighbours,
            exit_distance,
            exit_reachable_cache: HashMap::new(),
            reachable_positions_cache: HashMap::new(),
            reachable_laser_paths_cache: HashMap::new(),
        }
    }

    /// Ensure that `exit_reachable_cache[t]` is populated: the set of positions from which an
    /// exit can still be reached within `t_max - t` steps.
    fn ensure_exit_reachable(&mut self, t: usize) {
        if t > self.t_max || self.exit_reachable_cache.contains_key(&t) {
            return;
        }
        let remaining = self.t_max - t;
        let result: HashSet<Position> = self
            .exit_distance
            .iter()
            .filter(|&(_, &d)| d <= remaining)
            .map(|(&p, _)| p)
            .collect();
        self.exit_reachable_cache.insert(t, result);
    }

    /// Whether `pos` can still reach an exit within `t_max - t` steps. Assumes `update` has
    /// already been called for this `t`.
    fn can_reach_exit_at(&self, t: usize, pos: &Position) -> bool {
        match self.exit_reachable_cache.get(&t) {
            Some(positions) => positions.contains(pos),
            None => false,
        }
    }

    /// Ensure that `reachable_positions_cache[(agent, t)]` is populated, computing (and caching)
    /// every time step from the last cached one up to `t` along the way. Recursing this way (rather
    /// than holding a reference across the recursive call) lets each step borrow `&mut self` freely.
    fn ensure_reachable_positions(&mut self, agent: usize, t: usize) {
        if self.reachable_positions_cache.contains_key(&(agent, t)) {
            return;
        }
        if t > self.t_max || agent >= self.n_agents {
            self.reachable_positions_cache
                .insert((agent, t), HashSet::with_capacity(0));
            return;
        }

        let result = if t == 0 {
            let mut set = HashSet::with_capacity(self.n_agents);
            set.insert(self.start_pos[agent]);
            set
        } else {
            self.ensure_reachable_positions(agent, t - 1);
            let mut next = HashSet::new();
            for &pos in &self.reachable_positions_cache[&(agent, t - 1)] {
                for &n in &self.neighbours[&pos] {
                    next.insert(n);
                }
            }
            next
        };
        self.ensure_exit_reachable(t);
        let positions_in_reach_of_exit = &self.exit_reachable_cache[&t];
        let filtered = &result & positions_in_reach_of_exit;
        self.reachable_positions_cache.insert((agent, t), filtered);
    }

    /// Ensure that `reachable_laser_paths_cache[(laser_idx, t)]` is populated.
    fn ensure_reachable_laser_path(&mut self, laser_idx: usize, t: usize) {
        if self.reachable_laser_paths_cache.contains_key(&(laser_idx, t)) {
            return;
        }
        if laser_idx >= self.laser_sources.len() || t > self.t_max {
            self.reachable_laser_paths_cache.insert((laser_idx, t), Vec::new());
            return;
        }

        let agent_id = self.laser_sources[laser_idx].agent_id;
        self.ensure_reachable_positions(agent_id, t);
        let blockable = &self.reachable_positions_cache[&(agent_id, t)];
        let result: Vec<Position> = self.laser_sources[laser_idx]
            .path
            .iter()
            .copied()
            .filter(|p| blockable.contains(p))
            .collect();
        self.reachable_laser_paths_cache.insert((laser_idx, t), result);
    }

    /// Pre-compute (and cache) every piece of on-demand data needed to generate constraints
    /// for time step `t`: reachable positions for each agent (including the lookahead step
    /// `t + 1` needed by `can_stay`) and reachable laser paths for each source. Must be called
    /// before any of the `reachable_*`/`can_*`/`prev_neighbours` accessors for this `t`.
    pub fn update(&mut self, t: usize) {
        let lookahead = (t + 1).min(self.t_max);
        for agent in 0..self.n_agents {
            self.ensure_reachable_positions(agent, lookahead);
        }
        for laser_idx in 0..self.laser_sources.len() {
            self.ensure_reachable_laser_path(laser_idx, t);
        }
    }

    /// Reachable positions for a single agent at time `t`, returning a reference to the cached
    /// set (no cloning). Assumes `update` has already been called for this `t`.
    pub fn reachable_positions_for_agent(&self, agent: usize, t: usize) -> &HashSet<Position> {
        &self.reachable_positions_cache[&(agent, t)]
    }

    /// Positions reachable by *all* the given agents exactly at time `t` (already filtered by
    /// exit-reachability). Mirrors `ConstraintContext.reachable_positions`. Assumes `update` has
    /// already been called for this `t`.
    pub fn reachable_positions(&self, t: usize, agents: &[usize]) -> HashSet<Position> {
        if agents.is_empty() {
            return HashSet::new();
        }
        let mut reachable = self.reachable_positions_for_agent(agents[0], t).clone();
        for &agent in &agents[1..] {
            let agent_reachable = self.reachable_positions_for_agent(agent, t);
            reachable.retain(|p| agent_reachable.contains(p));
        }
        reachable
    }

    /// Whether staying in `pos` for one more time step (from `t` to `t + 1`) is still
    /// compatible with eventually reaching an exit. Assumes `update` has already been called
    /// for this `t`.
    pub fn can_stay(&self, t: usize, pos: Position) -> bool {
        if t + 1 > self.t_max {
            return false;
        }
        self.can_reach_exit_at(t + 1, &pos)
    }

    /// Positions the agent could have occupied at time `t - 1` to reach `(i, j)` at `t`.
    /// Assumes `update` has already been called for this `t`.
    pub fn prev_neighbours(&self, agent: usize, pos: &Position, t: usize) -> Vec<Position> {
        if t == 0 {
            return Vec::new();
        }
        let reachable = &self.reachable_positions_cache[&(agent, t - 1)];
        self.predecessors[pos]
            .iter()
            .copied()
            .filter(|p| reachable.contains(p))
            .collect()
    }

    /// The reachable laser path for a given laser source at time `t`: the beam tiles that can
    /// still be blocked. Assumes `update` has already been called for this `t`.
    pub fn get_reachable_laser_path(&self, laser_idx: usize, t: usize) -> &Vec<Position> {
        &self.reachable_laser_paths_cache[&(laser_idx, t)]
    }
}

fn compute_exit_distance(
    exits: &HashSet<Position>,
    predecessors: &HashMap<Position, Vec<Position>>,
) -> HashMap<Position, usize> {
    let mut dist: HashMap<Position, usize> = exits.iter().map(|&p| (p, 0)).collect();
    let mut frontier: VecDeque<Position> = exits.iter().copied().collect();
    while let Some(current) = frontier.pop_front() {
        let current_dist = dist[&current];
        if let Some(preds) = predecessors.get(&current) {
            for &pred in preds {
                if !dist.contains_key(&pred) {
                    dist.insert(pred, current_dist + 1);
                    frontier.push_back(pred);
                }
            }
        }
    }
    dist
}
