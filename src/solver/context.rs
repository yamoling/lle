use std::collections::{HashMap, HashSet, VecDeque};

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

/// Pre-computed data shared across all constraint-generation routines, mirroring the
/// Python `ConstraintContext`. Built once per `(world, t_max)` pair.
pub struct ConstraintContext {
    pub t_max: usize,
    pub n_agents: usize,
    pub start_pos: Vec<Pos>,
    pub exits: HashSet<Pos>,

    pub predecessors: HashMap<Pos, Vec<Pos>>,

    /// `reachable_positions[agent][t]`: positions reachable by `agent` exactly at time `t`,
    /// already filtered by exit-reachability.
    reachable_positions: Vec<Vec<HashSet<Pos>>>,

    /// `exit_reachable[t]`: positions from which an exit can still be reached within `t_max - t` steps.
    exit_reachable: Vec<HashSet<Pos>>,

    pub solution_lower_bound: usize,

    pub laser_sources: Vec<LaserSourceInfo>,
    /// `reachable_laser_paths[laser_idx][t]`: subset of the beam path reachable (blockable) at time `t`,
    /// in beam order. Indexed in the same order as `laser_sources`.
    pub reachable_laser_paths: Vec<Vec<Vec<Pos>>>,
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

        // exit_reachable[t] = positions from which an exit can still be reached within (t_max - t) steps
        let exit_reachable: Vec<HashSet<Pos>> = (0..=t_max)
            .map(|t| {
                let remaining = t_max - t;
                exit_distance
                    .iter()
                    .filter(|&(_, &d)| d <= remaining)
                    .map(|(&p, _)| p)
                    .collect()
            })
            .collect();

        let solution_lower_bound = start_pos
            .iter()
            .map(|p| exit_distance.get(p).copied().unwrap_or(0))
            .max()
            .unwrap_or(0);

        // Time-wise reachability per agent (forward flood fill), filtered by exit-reachability.
        let mut reachable_positions: Vec<Vec<HashSet<Pos>>> = Vec::with_capacity(n_agents);
        for &start in &start_pos {
            let mut reachable: Vec<HashSet<Pos>> = Vec::with_capacity(t_max + 1);
            reachable.push(HashSet::from([start]));
            for _ in 0..t_max {
                let frontier = reachable.last().unwrap();
                let mut next = HashSet::new();
                for &pos in frontier {
                    for &n in &neighbours[&pos] {
                        next.insert(n);
                    }
                }
                reachable.push(next);
            }
            // Filter by exit-reachability at each time step.
            for (t, set) in reachable.iter_mut().enumerate() {
                set.retain(|p| exit_reachable[t].contains(p));
            }
            reachable_positions.push(reachable);
        }

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

        let mut ctx = ConstraintContext {
            t_max,
            n_agents,
            start_pos,
            exits,
            predecessors,
            reachable_positions,
            exit_reachable,
            solution_lower_bound,
            laser_sources,
            reachable_laser_paths: Vec::new(),
        };

        ctx.reachable_laser_paths = ctx
            .laser_sources
            .iter()
            .map(|source| {
                (0..=t_max)
                    .map(|t| {
                        let blockable = ctx.reachable_positions(t, &[source.agent_id]);
                        source
                            .path
                            .iter()
                            .copied()
                            .filter(|p| blockable.contains(p))
                            .collect()
                    })
                    .collect()
            })
            .collect();

        ctx
    }

    /// Positions reachable by *all* the given agents exactly at time `t` (already filtered by
    /// exit-reachability). Mirrors `ConstraintContext.reachable_positions`.
    pub fn reachable_positions(&self, t: usize, agents: &[usize]) -> HashSet<Pos> {
        if t > self.t_max || agents.is_empty() {
            return HashSet::new();
        }
        let mut reachable = self.reachable_positions[agents[0]][t].clone();
        for &agent in &agents[1..] {
            reachable.retain(|p| self.reachable_positions[agent][t].contains(p));
        }
        reachable
    }

    /// Whether staying in `pos` for one more time step (from `t` to `t + 1`) is still
    /// compatible with eventually reaching an exit.
    pub fn can_stay(&self, t: usize, pos: Pos) -> bool {
        if t + 1 > self.t_max {
            return false;
        }
        self.exit_reachable[t + 1].contains(&pos)
    }

    /// Positions the agent could have occupied at time `t - 1` to reach `(i, j)` at `t`.
    pub fn prev_neighbours(&self, agent: usize, pos: Pos, t: usize) -> Vec<Pos> {
        if t == 0 {
            return Vec::new();
        }
        let reachable = self.reachable_positions(t - 1, &[agent]);
        self.predecessors[&pos]
            .iter()
            .copied()
            .filter(|p| reachable.contains(p))
            .collect()
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
