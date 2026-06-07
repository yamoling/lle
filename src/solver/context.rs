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
    updated_until: usize,

    // Neighborhood and distance information (computed once at construction).
    neighbours: HashMap<Position, Vec<Position>>,
    /// `distance_buckets[d]` = positions whose distance to the nearest exit is exactly `d`
    /// (only for `d <= t_max`, the only distances that ever matter). Used to incrementally
    /// shrink `exit_reachable` as `t` grows, instead of recomputing it from scratch every step.
    distance_buckets: Vec<Vec<Position>>,

    // Cached on-demand data (lazily computed per time step, in increasing order of `t`).
    /// Positions from which an exit can still be reached within `t_max - t` steps, for the
    /// current `updated_until`. This set only shrinks as `t` grows, so it is maintained
    /// incrementally rather than cached per time step.
    exit_reachable: HashSet<Position>,

    /// Cache for reachable positions per agent and time step, flattened as `agent * (t_max + 1)
    /// + t` (dense, since every `(agent, t)` in `0..n_agents x 0..=t_max` is eventually
    /// populated in increasing order of `t`). Avoids hashing `(usize, usize)` keys on every
    /// lookup in the hot path.
    reachable_positions_cache: Vec<HashSet<Position>>,

    /// Cache for reachable laser paths per laser source and time step, flattened the same way
    /// as `reachable_positions_cache` (`laser_idx * (t_max + 1) + t`).
    reachable_laser_paths_cache: Vec<Vec<Position>>,
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
        // Bucket positions by their exact distance to the nearest exit (capped at `t_max`,
        // since farther positions can never be exit-reachable within the horizon) and seed
        // `exit_reachable` with the full set for `t = 0` (i.e. `remaining = t_max`).
        let mut distance_buckets: Vec<Vec<Position>> = vec![Vec::new(); t_max + 1];
        let mut exit_reachable = HashSet::with_capacity(exit_distance.len());
        for (&pos, &d) in &exit_distance {
            if d <= t_max {
                distance_buckets[d].push(pos);
                exit_reachable.insert(pos);
            }
        }

        // Dense per-`(agent, t)` / `(laser_idx, t)` caches, flattened as `key * (t_max + 1) + t`.
        // Every slot is eventually populated (in increasing order of `t`), so a flat `Vec`
        // avoids hashing `(usize, usize)` keys on every lookup in the hot constraint-generation
        // loop. Seed the `t = 0` slots here; `update` fills in the rest.
        let stride = t_max + 1;
        let mut reachable_positions_cache = vec![HashSet::new(); n_agents * stride];
        for agent in 0..n_agents {
            reachable_positions_cache[agent * stride] = HashSet::from([start_pos[agent]]);
        }
        let reachable_laser_paths_cache = vec![Vec::new(); laser_sources.len() * stride];

        ConstraintContext {
            t_max,
            n_agents,
            start_pos,
            exits,
            predecessors,
            solution_lower_bound,
            laser_sources,
            neighbours,
            distance_buckets,
            updated_until: 0,
            exit_reachable,
            reachable_positions_cache,
            reachable_laser_paths_cache,
        }
    }

    /// Shrink `exit_reachable` from the set valid at `t - 1` to the set valid at `t`: as `t`
    /// grows by one, the remaining horizon `t_max - t` shrinks by one, so exactly the positions
    /// at distance `t_max - t + 1` fall out of reach. `t` must be processed in increasing order
    /// (guaranteed by `update`), so the set is shrunk in place rather than recomputed.
    /// Flat index into `reachable_positions_cache` for `(agent, t)`.
    #[inline]
    fn pos_cache_idx(&self, agent: usize, t: usize) -> usize {
        agent * (self.t_max + 1) + t
    }

    /// Flat index into `reachable_laser_paths_cache` for `(laser_idx, t)`.
    #[inline]
    fn laser_path_cache_idx(&self, laser_idx: usize, t: usize) -> usize {
        laser_idx * (self.t_max + 1) + t
    }

    fn update_exit_reachable(&mut self, t: usize) {
        let excluded_distance = self.t_max - t + 1;
        for pos in &self.distance_buckets[excluded_distance] {
            self.exit_reachable.remove(pos);
        }
    }

    /// Compute and cache `reachable_positions_cache[agent, t]` from the `t - 1` entry.
    fn update_reachable_positions(&mut self, agent: usize, t: usize) {
        let mut result = HashSet::new();
        for &pos in &self.reachable_positions_cache[self.pos_cache_idx(agent, t - 1)] {
            for &n in &self.neighbours[&pos] {
                result.insert(n);
            }
        }
        result.retain(|p| self.exit_reachable.contains(p));
        let idx = self.pos_cache_idx(agent, t);
        self.reachable_positions_cache[idx] = result;
    }

    /// Compute and cache `reachable_laser_paths_cache[laser_idx, t]`.
    fn update_reachable_laser_path(&mut self, laser_idx: usize, t: usize) {
        let agent_id = self.laser_sources[laser_idx].agent_id;
        let blockable = &self.reachable_positions_cache[self.pos_cache_idx(agent_id, t)];
        let result: Vec<Position> = self.laser_sources[laser_idx]
            .path
            .iter()
            .copied()
            .filter(|p| blockable.contains(p))
            .collect();
        let idx = self.laser_path_cache_idx(laser_idx, t);
        self.reachable_laser_paths_cache[idx] = result;
    }

    /// Pre-compute (and cache) every piece of on-demand data needed to generate constraints
    /// for time step `t`: reachable positions for each agent and reachable laser paths for each source.
    pub fn update(&mut self, t: usize) {
        if t <= self.updated_until {
            return;
        }
        for tt in (self.updated_until + 1)..=t {
            self.update_exit_reachable(tt);
            for agent in 0..self.n_agents {
                self.update_reachable_positions(agent, tt);
            }
            for laser_idx in 0..self.laser_sources.len() {
                self.update_reachable_laser_path(laser_idx, tt);
            }
        }
        self.updated_until = t;
    }

    /// Reachable positions for a single agent at time `t`, returning a reference to the cached
    /// set (no cloning). Assumes `update` has already been called for this `t`.
    pub fn reachable_positions_for_agent(&self, agent: usize, t: usize) -> &HashSet<Position> {
        &self.reachable_positions_cache[self.pos_cache_idx(agent, t)]
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

    /// Positions the agent could have occupied at time `t - 1` to reach `(i, j)` at `t`.
    /// Assumes `update` has already been called for this `t`.
    pub fn prev_neighbours(&self, agent: usize, pos: &Position, t: usize) -> Vec<Position> {
        if t == 0 {
            return Vec::new();
        }
        let reachable = &self.reachable_positions_cache[self.pos_cache_idx(agent, t - 1)];
        self.predecessors[pos]
            .iter()
            .copied()
            .filter(|p| reachable.contains(p))
            .collect()
    }

    /// The reachable laser path for a given laser source at time `t`: the beam tiles that can
    /// still be blocked. Assumes `update` has already been called for this `t`.
    pub fn get_reachable_laser_path(&self, laser_idx: usize, t: usize) -> &Vec<Position> {
        &self.reachable_laser_paths_cache[self.laser_path_cache_idx(laser_idx, t)]
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
