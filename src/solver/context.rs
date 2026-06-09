use std::collections::{HashMap, HashSet, VecDeque};

use strum::IntoEnumIterator;

use super::position_set::PositionSet;
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

/// Data shared across all constraint-generation routines.
/// Built once per `(world, t_max)` pair. Data is computed on-demand and cached for efficiency.
pub struct ConstraintContext {
    pub t_max: usize,
    pub n_agents: usize,
    pub start_pos: Vec<Position>,
    /// `predecessors[i * width + j]` = positions from which an agent can move into `(i, j)`.
    pub predecessors: Vec<Vec<Position>>,
    pub solution_lower_bound: usize,
    pub laser_sources: Vec<LaserSourceInfo>,
    height: usize,
    width: usize,
    updated_until: usize,

    // Neighborhood and distance information (computed once at construction), flat-indexed by
    // `i * width + j` rather than hashed by `Position`: positions are dense and bounded, so a
    // plain array lookup beats hashing in these hot paths.
    /// `neighbours[i * width + j]` = `[(i, j), ...reachable single-step neighbours]`.
    neighbours: Vec<Vec<Position>>,

    // Cached on-demand data (lazily computed per time step, in increasing order of `t`).
    /// `distance_buckets[d]` = positions whose distance to the nearest exit is exactly `d`
    /// (only for `d <= t_max`, the only distances that ever matter). Used to incrementally
    /// build `exit_reachable` as `t` grows, instead of recomputing each entry from scratch.
    distance_buckets: Vec<Vec<Position>>,

    /// `exit_reachable[t]` = positions from which an exit can still be reached within
    /// `t_max - t` steps. Lazily computed and cached per time step (in increasing order of
    /// `t`, like `relevant_positions`): each entry is independent and never overwritten once
    /// computed.
    exit_reachable: Vec<PositionSet>,

    /// Cache for reachable positions per agent and time step, flattened as `agent * (t_max + 1)
    /// + t` (dense, since every `(agent, t)` in `0..n_agents x 0..=t_max` is eventually
    /// populated in increasing order of `t`). Bitsets here turn the per-agent intersections in
    /// `reachable_positions` into word-at-a-time ANDs instead of per-element hashing.
    relevant_positions: Vec<PositionSet>,

    /// Cache for reachable laser paths per laser source and time step, flattened the same way
    /// as `reachable_positions` (`laser_idx * (t_max + 1) + t`).
    relevant_laser_paths: Vec<Vec<Position>>,
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

        let flat_index = |pos: Position| pos.i * width + pos.j;

        // neighbours[i * width + j] = [(i, j), ...reachable single-step neighbours]
        let mut neighbours: Vec<Vec<Position>> = vec![Vec::new(); height * width];
        for &pos in &valid_positions {
            let mut succ = vec![pos];
            for n in neighbours_of(pos, &exits, height, width, &walls) {
                if valid_positions.contains(&n) {
                    succ.push(n);
                }
            }
            neighbours[flat_index(pos)] = succ;
        }

        // Reverse adjacency: predecessors[p] = positions from which an agent can move into p.
        let mut predecessors: Vec<Vec<Position>> = vec![Vec::new(); height * width];
        for &pos in &valid_positions {
            for &succ in &neighbours[flat_index(pos)] {
                predecessors[flat_index(succ)].push(pos);
            }
        }

        let exit_distance = compute_exit_distance(&exits, &predecessors, width);
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
        // since farther positions can never be exit-reachable within the horizon).
        let mut distance_buckets: Vec<Vec<Position>> = vec![Vec::new(); t_max + 1];
        for (&pos, &d) in &exit_distance {
            if d <= t_max {
                distance_buckets[d].push(pos);
            }
        }

        // Seed `exit_reachable[0]` with the full set of exit-reachable positions (distance
        // `<= t_max`, i.e. `remaining = t_max - 0 = t_max`); `update` fills in the rest by
        // shrinking this set one distance bucket at a time as `t` grows.
        let mut exit_reachable: Vec<PositionSet> =
            vec![PositionSet::empty(height, width); t_max + 1];
        for bucket in &distance_buckets {
            for &pos in bucket {
                exit_reachable[0].insert(pos);
            }
        }

        // Dense per-`(agent, t)` / `(laser_idx, t)` caches, flattened as `key * (t_max + 1) + t`.
        // Every slot is eventually populated (in increasing order of `t`), so a flat `Vec`
        // avoids hashing `(usize, usize)` keys on every lookup in the hot constraint-generation
        // loop. Seed the `t = 0` slots here; `update` fills in the rest.
        let stride = t_max + 1;
        let mut relevant_positions = vec![PositionSet::empty(height, width); n_agents * stride];
        for agent in 0..n_agents {
            // Only the initial positions are relevant to consider at t=0
            let agent_start = start_pos[agent];
            if exit_reachable[0].contains(&agent_start) {
                relevant_positions[agent * stride] =
                    PositionSet::singleton(height, width, start_pos[agent]);
            }
        }
        // Seed the `t = 0` laser-path slots from the `t = 0` reachable-positions seed above
        // (mirroring `update_reachable_laser_path`); `update` only fills in `t >= 1`, since its
        // loop range `(updated_until + 1)..=t` is empty for `t = 0`.
        let mut relevant_laser_paths = vec![Vec::new(); laser_sources.len() * stride];
        for (laser_idx, source) in laser_sources.iter().enumerate() {
            let blockable = &relevant_positions[source.agent_id * stride];
            relevant_laser_paths[laser_idx * stride] = source
                .path
                .iter()
                .filter(|p| blockable.contains(p))
                .copied()
                .collect();
        }

        ConstraintContext {
            t_max,
            n_agents,
            start_pos,
            predecessors,
            solution_lower_bound,
            laser_sources,
            height,
            width,
            neighbours,
            updated_until: 0,
            distance_buckets,
            exit_reachable,
            relevant_positions,
            relevant_laser_paths,
        }
    }

    /// Flat index into `neighbours` / `predecessors` for `pos`.
    #[inline]
    fn flat_index(&self, pos: &Position) -> usize {
        pos.i * self.width + pos.j
    }

    /// Flat index into `reachable_positions_cache` for `(agent, t)`.
    #[inline]
    fn pos_cache_idx(&self, agent: usize, t: usize) -> usize {
        agent * (self.t_max + 1) + t
    }

    /// Flat index into `reachable_laser_paths_cache` for `(laser_idx, t)`.
    #[inline]
    fn laser_path_cache_idx(&self, laser_id: usize, t: usize) -> usize {
        laser_id * (self.t_max + 1) + t
    }

    /// Compute and cache `exit_reachable[t]` from `exit_reachable[t - 1]`: as `t` grows by one,
    /// the remaining horizon `t_max - t` shrinks by one, so exactly the positions at distance
    /// `t_max - t + 1` fall out of reach. Clones the previous entry and removes that bucket,
    /// rather than mutating it in place, so every `exit_reachable[t]` stays independently cached.
    fn update_exit_reachable(&mut self, t: usize) {
        let mut result = self.exit_reachable[t - 1].clone();
        let excluded_distance = self.t_max - t + 1;
        for pos in &self.distance_buckets[excluded_distance] {
            result.remove(pos);
        }
        self.exit_reachable[t] = result;
    }

    /// Update the relevant positions for each agent at time step t.
    /// # Details
    /// A position is relevant to a given agent at time step t if:
    ///     - the agent can reach it at time step t
    ///     - the agent can still access the exit within `t_max - t` steps
    fn update_relevant_positions(&mut self, t: usize) {
        for agent in 0..self.n_agents {
            let mut result = PositionSet::empty(self.height, self.width);
            for pos in &self.relevant_positions[self.pos_cache_idx(agent, t - 1)] {
                for &n in &self.neighbours[self.flat_index(&pos)] {
                    result.insert(n);
                }
            }
            result.intersect_with(&self.exit_reachable[t]);
            let idx = self.pos_cache_idx(agent, t);
            self.relevant_positions[idx] = result;
        }
    }

    /// Compute and cache `relevant_laser_paths_cache[..., t]`.
    ///
    /// # Details
    /// The relevant laser path for a given laser source at time `t` is the subset of its path that
    /// is still blockable by the agent that is using it.
    ///
    /// # Assumptions
    /// This function assumes that `update_relevant_positions` has already been called for `t`.
    fn update_relevant_laser_paths(&mut self, t: usize) {
        for laser_idx in 0..self.laser_sources.len() {
            let agent_id = self.laser_sources[laser_idx].agent_id;
            let blockable = &self.relevant_positions[self.pos_cache_idx(agent_id, t)];
            let result: Vec<Position> = self.laser_sources[laser_idx]
                .path
                .iter()
                .copied()
                .filter(|p| blockable.contains(p))
                .collect();
            let idx = self.laser_path_cache_idx(laser_idx, t);
            self.relevant_laser_paths[idx] = result;
        }
    }

    /// Pre-compute (and cache) every piece of on-demand data needed to generate constraints
    /// for time step `t`: reachable positions for each agent and reachable laser paths for each source.
    pub fn update(&mut self, t: usize) {
        if t <= self.updated_until {
            return;
        }
        for tt in (self.updated_until + 1)..=t {
            self.update_exit_reachable(tt);
            self.update_relevant_positions(tt);
            self.update_relevant_laser_paths(tt);
        }
        self.updated_until = t;
    }

    /// Relevant positions for a single agent at time `t`, i.e. positions that the agent
    /// can reach and from which it can still access the exit within due time.
    pub fn relevant_positions_for_agent(&self, agent: usize, t: usize) -> &PositionSet {
        &self.relevant_positions[self.pos_cache_idx(agent, t)]
    }

    /// Positions relevant to *all* the given agents exactly at time `t` (already filtered by
    /// exit-reachability), i.e. intersection of the positions relevant to each agent individually.
    ///
    /// See `relevant_positions_for_agent` for more details.
    pub fn relevant_positions(&self, t: usize, agents: &[usize]) -> PositionSet {
        if agents.is_empty() {
            return PositionSet::empty(self.height, self.width);
        }
        let mut reachable = self.relevant_positions_for_agent(agents[0], t).clone();
        for &agent in &agents[1..] {
            reachable.intersect_with(self.relevant_positions_for_agent(agent, t));
        }
        reachable
    }

    /// Positions the agent could have occupied at time `t - 1` to reach `(i, j)` at `t`.
    /// Assumes `update` has already been called for this `t`.
    pub fn prev_neighbours(&self, agent: usize, pos: &Position, t: usize) -> Vec<Position> {
        if t == 0 {
            return Vec::new();
        }
        let reachable = &self.relevant_positions[self.pos_cache_idx(agent, t - 1)];
        self.predecessors[self.flat_index(pos)]
            .iter()
            .copied()
            .filter(|p| reachable.contains(p))
            .collect()
    }

    /// The reachable laser path for a given laser source at time `t`: the beam tiles that can
    /// still be blocked. Assumes `update` has already been called for this `t`.
    pub fn get_relevant_laser_path(&self, laser_id: usize, t: usize) -> &Vec<Position> {
        &self.relevant_laser_paths[self.laser_path_cache_idx(laser_id, t)]
    }
}

#[cfg(test)]
impl ConstraintContext {
    /// ONLY USE THIS FOR TESTING PURPOSES
    ///
    /// `exit_reachable` is now lazily computed by `update`, so make sure every entry is
    /// available before scanning it for `pos`.
    fn get_exit_distance(&mut self, pos: &Position) -> usize {
        self.update(self.t_max);
        for t in (0..=self.t_max).rev() {
            if self.exit_reachable[t].contains(pos) {
                return self.t_max - t;
            }
        }
        panic!("{pos:?} is not in the PositionSet !")
    }

    /// ONLY USE THIS FOR TESTING PURPOSES
    fn is_exit_reachable(&self, pos: &Position, t: usize) -> bool {
        self.exit_reachable[t].contains(pos)
    }
}

fn compute_exit_distance(
    exits: &HashSet<Position>,
    predecessors: &[Vec<Position>],
    width: usize,
) -> HashMap<Position, usize> {
    let mut dist: HashMap<Position, usize> = exits.iter().map(|&p| (p, 0)).collect();
    let mut frontier: VecDeque<Position> = exits.iter().copied().collect();
    while let Some(current) = frontier.pop_front() {
        let current_dist = dist[&current];
        for &pred in &predecessors[current.i * width + current.j] {
            if !dist.contains_key(&pred) {
                dist.insert(pred, current_dist + 1);
                frontier.push_back(pred);
            }
        }
    }
    dist
}

#[cfg(test)]
#[path = "../unit_tests/test_context.rs"]
mod tests;
