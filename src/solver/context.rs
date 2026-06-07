use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Mutex;

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
    exit_reachable_cache: Mutex<HashMap<usize, HashSet<Position>>>,

    /// Cache for reachable positions per agent and time step.
    reachable_positions_cache: Mutex<HashMap<(usize, usize), HashSet<Position>>>,

    /// Cache for reachable laser paths per laser source and time step.
    reachable_laser_paths_cache: Mutex<HashMap<(usize, usize), Vec<Position>>>,
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
            exit_reachable_cache: Mutex::new(HashMap::new()),
            reachable_positions_cache: Mutex::new(HashMap::new()),
            reachable_laser_paths_cache: Mutex::new(HashMap::new()),
        }
    }

    /// Get or compute `exit_reachable[t]`: positions from which an exit can still be reached
    /// within `t_max - t` steps. Cached for efficiency.
    fn get_exit_reachable(&self, t: usize) -> HashSet<Position> {
        if t > self.t_max {
            return HashSet::new();
        }
        let mut cache = self.exit_reachable_cache.lock().unwrap();
        if let Some(result) = cache.get(&t) {
            return result.clone();
        }
        let remaining = self.t_max - t;
        let result: HashSet<Position> = self
            .exit_distance
            .iter()
            .filter(|&(_, &d)| d <= remaining)
            .map(|(&p, _)| p)
            .collect();
        cache.insert(t, result.clone());
        result
    }

    /// Get or compute reachable positions for a single agent at time `t`. Cached for efficiency.
    fn get_reachable_positions_for_agent(&self, agent: usize, t: usize) -> HashSet<Position> {
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
        let filtered: HashSet<Position> = result
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
    pub fn reachable_positions(&self, t: usize, agents: &[usize]) -> HashSet<Position> {
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
    pub fn can_stay(&self, t: usize, pos: Position) -> bool {
        if t + 1 > self.t_max {
            return false;
        }
        self.get_exit_reachable(t + 1).contains(&pos)
    }

    /// Positions the agent could have occupied at time `t - 1` to reach `(i, j)` at `t`.
    pub fn prev_neighbours(&self, agent: usize, pos: Position, t: usize) -> Vec<Position> {
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
    pub fn get_reachable_laser_path(&self, laser_idx: usize, t: usize) -> Vec<Position> {
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
        let result: Vec<Position> = source
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
