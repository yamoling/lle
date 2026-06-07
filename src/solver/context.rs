use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Mutex;

use strum::IntoEnumIterator;

use crate::{World, tiles::Direction};

/// A grid position as (row, column), matching the Python solver's `(i, j)` convention.
pub type Pos = (usize, usize);

fn neighbours_of(
    pos: Pos,
    exits: &HashSet<Pos>,
    height: usize,
    width: usize,
    walls: &HashSet<Pos>,
) -> Vec<Pos> {
    if exits.contains(&pos) {
        // Once an agent reaches an exit, it can no longer move.
        return vec![pos];
    }
    let (i, j) = pos;
    let mut result = Vec::new();
    for d in Direction::iter() {
        let (di, dj) = d.delta();
        let ni = i as i32 + di;
        let nj = j as i32 + dj;
        if ni >= 0 && nj >= 0 {
            let (ni, nj) = (ni as usize, nj as usize);
            if ni < height && nj < width && !walls.contains(&(ni, nj)) {
                result.push((ni, nj));
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
    pub path: Vec<Pos>,
}

/// Data shared across all constraint-generation routines, mirroring the Python `ConstraintContext`.
/// Built once per `(world, t_max)` pair. Expensive data like time-wise reachability is computed
/// on-demand and cached for efficiency.
pub struct ConstraintContext {
    pub t_max: usize,
    pub n_agents: usize,
    pub start_pos: Vec<Pos>,
    pub exits: HashSet<Pos>,
    pub predecessors: HashMap<Pos, Vec<Pos>>,
    pub solution_lower_bound: usize,
    pub laser_sources: Vec<LaserSourceInfo>,

    // Neighborhood and distance information (computed once at construction).
    neighbours: HashMap<Pos, Vec<Pos>>,
    exit_distance: HashMap<Pos, usize>,

    // Cached on-demand data (lazily computed per time step).
    /// Cache for `exit_reachable[t]`: positions from which an exit can still be reached
    /// within `t_max - t` steps.
    exit_reachable_cache: Mutex<HashMap<usize, HashSet<Pos>>>,

    /// Cache for reachable positions per agent and time step.
    reachable_positions_cache: Mutex<HashMap<(usize, usize), HashSet<Pos>>>,

    /// Cache for reachable laser paths per laser source and time step.
    reachable_laser_paths_cache: Mutex<HashMap<(usize, usize), Vec<Pos>>>,
}

impl ConstraintContext {
    pub fn new(world: &World, t_max: usize) -> Self {
        let height = world.height();
        let width = world.width();
        let n_agents = world.n_agents();
        let walls: HashSet<Pos> = world.walls().into_iter().map(|p| p.as_ij()).collect();
        let voids: HashSet<Pos> = world
            .void_positions()
            .into_iter()
            .map(|p| p.as_ij())
            .collect();
        let exits: HashSet<Pos> = world
            .exits_positions()
            .into_iter()
            .map(|p| p.as_ij())
            .collect();
        let start_pos: Vec<Pos> = world.starts().iter().map(|p| p.as_ij()).collect();

        let mut valid_positions = HashSet::new();
        for i in 0..height {
            for j in 0..width {
                let pos = (i, j);
                if !walls.contains(&pos) && !voids.contains(&pos) {
                    valid_positions.insert(pos);
                }
            }
        }

        // neighbours[pos] = [pos, ...reachable single-step neighbours]
        let mut neighbours: HashMap<Pos, Vec<Pos>> = HashMap::new();
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
        let mut predecessors: HashMap<Pos, Vec<Pos>> =
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
            let (di, dj) = source.direction().delta();
            let (di, dj) = (di as i64, dj as i64);
            let mut i = pos.i as i64 + di;
            let mut j = pos.j as i64 + dj;
            let mut path = Vec::new();
            while i >= 0
                && j >= 0
                && (i as usize) < height
                && (j as usize) < width
                && !walls.contains(&(i as usize, j as usize))
            {
                path.push((i as usize, j as usize));
                i += di;
                j += dj;
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
            exit_reachable_cache: Mutex::new(HashMap::new()),
            reachable_positions_cache: Mutex::new(HashMap::new()),
            reachable_laser_paths_cache: Mutex::new(HashMap::new()),
        }
    }

    /// Get or compute `exit_reachable[t]`: positions from which an exit can still be reached
    /// within `t_max - t` steps. Cached for efficiency.
    fn get_exit_reachable(&self, t: usize) -> HashSet<Pos> {
        if t > self.t_max {
            return HashSet::new();
        }
        let mut cache = self.exit_reachable_cache.lock().unwrap();
        if let Some(result) = cache.get(&t) {
            return result.clone();
        }
        let remaining = self.t_max - t;
        let result: HashSet<Pos> = self
            .exit_distance
            .iter()
            .filter(|&(_, &d)| d <= remaining)
            .map(|(&p, _)| p)
            .collect();
        cache.insert(t, result.clone());
        result
    }

    /// Get or compute reachable positions for a single agent at time `t`. Cached for efficiency.
    fn get_reachable_positions_for_agent(&self, agent: usize, t: usize) -> HashSet<Pos> {
        if t > self.t_max || agent >= self.n_agents {
            return HashSet::new();
        }
        let cache = self.reachable_positions_cache.lock().unwrap();
        if let Some(result) = cache.get(&(agent, t)) {
            return result.clone();
        }
        drop(cache); // Drop the lock before recursive call.

        let result = if t == 0 {
            let mut set = HashSet::new();
            set.insert(self.start_pos[agent]);
            set
        } else {
            let prev = self.get_reachable_positions_for_agent(agent, t - 1);
            let mut next = HashSet::new();
            for &pos in &prev {
                for &n in &self.neighbours[&pos] {
                    next.insert(n);
                }
            }
            next
        };

        // Filter by exit-reachability at this time step.
        let exit_reachable = self.get_exit_reachable(t);
        let filtered: HashSet<Pos> = result
            .into_iter()
            .filter(|p| exit_reachable.contains(p))
            .collect();

        self.reachable_positions_cache
            .lock()
            .unwrap()
            .insert((agent, t), filtered.clone());
        filtered
    }

    /// Positions reachable by *all* the given agents exactly at time `t` (already filtered by
    /// exit-reachability). Mirrors `ConstraintContext.reachable_positions`.
    pub fn reachable_positions(&self, t: usize, agents: &[usize]) -> HashSet<Pos> {
        if agents.is_empty() {
            return HashSet::new();
        }
        let mut reachable = self.get_reachable_positions_for_agent(agents[0], t);
        for &agent in &agents[1..] {
            let agent_reachable = self.get_reachable_positions_for_agent(agent, t);
            reachable.retain(|p| agent_reachable.contains(p));
        }
        reachable
    }

    /// Whether staying in `pos` for one more time step (from `t` to `t + 1`) is still
    /// compatible with eventually reaching an exit.
    pub fn can_stay(&self, t: usize, pos: Pos) -> bool {
        if t + 1 > self.t_max {
            return false;
        }
        self.get_exit_reachable(t + 1).contains(&pos)
    }

    /// Positions the agent could have occupied at time `t - 1` to reach `(i, j)` at `t`.
    pub fn prev_neighbours(&self, agent: usize, pos: Pos, t: usize) -> Vec<Pos> {
        if t == 0 {
            return Vec::new();
        }
        let reachable = self.get_reachable_positions_for_agent(agent, t - 1);
        self.predecessors[&pos]
            .iter()
            .copied()
            .filter(|p| reachable.contains(p))
            .collect()
    }

    /// Get or compute the reachable laser path for a given laser source at time `t`.
    /// This is called by the constraint generator to determine which beam tiles can be blocked.
    pub fn get_reachable_laser_path(&self, laser_idx: usize, t: usize) -> Vec<Pos> {
        if laser_idx >= self.laser_sources.len() || t > self.t_max {
            return Vec::new();
        }
        let cache = self.reachable_laser_paths_cache.lock().unwrap();
        if let Some(result) = cache.get(&(laser_idx, t)) {
            return result.clone();
        }
        drop(cache);

        let source = &self.laser_sources[laser_idx];
        let blockable = self.get_reachable_positions_for_agent(source.agent_id, t);
        let result: Vec<Pos> = source
            .path
            .iter()
            .copied()
            .filter(|p| blockable.contains(p))
            .collect();

        self.reachable_laser_paths_cache
            .lock()
            .unwrap()
            .insert((laser_idx, t), result.clone());
        result
    }
}

fn compute_exit_distance(
    exits: &HashSet<Pos>,
    predecessors: &HashMap<Pos, Vec<Pos>>,
) -> HashMap<Pos, usize> {
    let mut dist: HashMap<Pos, usize> = exits.iter().map(|&p| (p, 0)).collect();
    let mut frontier: VecDeque<Pos> = exits.iter().copied().collect();
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
