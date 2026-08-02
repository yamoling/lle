use std::collections::{HashMap, HashSet, VecDeque};

use strum::IntoEnumIterator;

use super::position_set::{Intersection, PositionSet};
use crate::Position;
use crate::{World, tiles::Direction};

fn neighbours_of(
    pos: Position,
    exits: &PositionSet,
    height: usize,
    width: usize,
    walls: &PositionSet,
) -> PositionSet {
    let mut result = PositionSet::empty(height, width);
    if exits.contains(&pos) {
        // Once an agent reaches an exit, it can no longer move.
        return result;
    }
    for d in Direction::iter() {
        if let Ok(n) = pos + d
            && n.i < height
            && n.j < width
            && !walls.contains(&n)
        {
            result.insert(n);
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
    /// `predecessors[i][j]` = positions from which an agent can move into `(i, j)`.
    pub predecessors: Vec<Vec<PositionSet>>,
    pub solution_lower_bound: usize,
    pub laser_sources: Vec<LaserSourceInfo>,
    exits: PositionSet,
    height: usize,
    width: usize,
    updated_until: Option<usize>,

    /// `neighbours[i][j]` = `[(i, j), ...reachable single-step neighbours]`.
    pub(crate) neighbours: Vec<Vec<PositionSet>>,

    /// `distance_buckets[d]` = positions whose distance to the nearest exit is exactly `d`
    /// (only for `d <= t_max`, the only distances that ever matter). Used to incrementally
    /// build `exit_reachable` as `t` grows, instead of recomputing each entry from scratch.
    distance_buckets: Vec<PositionSet>,

    /// `exit_reachable[t]` = positions from which an exit can still be reached within
    /// the remaining time budget (`t_max - t` steps).
    /// Lazily computed and cached per time step (in increasing order of `t`).
    exit_reachable: Vec<PositionSet>,

    /// Cache for reachable positions per agent and time step: `relevant_positions[agent][t]`.
    /// Bitsets here turn the per-agent intersections in `reachable_positions` into
    /// word-at-a-time ANDs instead of per-element hashing.
    relevant_positions: Vec<Vec<PositionSet>>,

    /// Cache for reachable laser paths per laser source and time step: `relevant_laser_paths[laser_idx][t]`.
    relevant_laser_paths: Vec<Vec<PositionSet>>,

    /// `forbidden_first_beam_tiles[agent]` = first beam tiles of every laser NOT owned by `agent`.
    /// A non-owner can never stand on the first beam tile: if the owner can reach it, the beam is
    /// active ↔ ¬owner, so non-owner requires owner present — impossible by no_overlap; otherwise
    /// the beam is constant-active and the non-owner dies. This is pre-computed once and applied at
    /// every time step during `update_relevant_positions`.
    forbidden_first_beam_tiles: Vec<PositionSet>,
}

impl ConstraintContext {
    pub fn new(world: &World, t_max: usize) -> Self {
        let height = world.height();
        let width = world.width();
        let n_agents = world.n_agents();
        let walls = PositionSet::from_positions(height, width, world.walls().into_iter());
        let voids = PositionSet::from_positions(height, width, world.void_positions().into_iter());
        let exits = PositionSet::from_positions(height, width, world.exits_positions().into_iter());
        // let exits: HashSet<Position> = exit_positions.iter().collect();
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

        // neighbours[i][j] = [(i, j), ...reachable single-step neighbours].
        // Invalid tiles stay empty; valid tiles get their self-neighbour from `neighbours_of`.
        let mut neighbours = vec![vec![PositionSet::empty(height, width); width]; height];

        for &pos in &valid_positions {
            neighbours[pos.i][pos.j].insert(pos);
            for n in neighbours_of(pos, &exits, height, width, &walls) {
                if valid_positions.contains(&n) {
                    neighbours[pos.i][pos.j].insert(n);
                }
            }
        }

        // Reverse adjacency: predecessors[i][j] = positions from which an agent can move into (i, j).
        let mut predecessors = vec![vec![PositionSet::empty(height, width); width]; height];
        for &pos in &valid_positions {
            for succ in &neighbours[pos.i][pos.j] {
                predecessors[succ.i][succ.j].insert(pos);
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
        // Opt 3: pre-compute first beam tiles forbidden for non-owner agents.
        let mut forbidden_first_beam_tiles: Vec<PositionSet> =
            vec![PositionSet::empty(height, width); n_agents];
        for source in &laser_sources {
            if let Some(&first_tile) = source.path.first() {
                for (agent, forbidden) in forbidden_first_beam_tiles.iter_mut().enumerate() {
                    if agent != source.agent_id {
                        forbidden.insert(first_tile);
                    }
                }
            }
        }

        // Bucket positions by their exact distance to the nearest exit (capped at `t_max`,
        // since farther positions can never be exit-reachable within the horizon).
        let mut distance_buckets = vec![PositionSet::empty(height, width); t_max + 1];
        for (&pos, &d) in &exit_distance {
            if d <= t_max {
                distance_buckets[d].insert(pos);
            }
        }

        let exit_reachable: Vec<PositionSet> = vec![PositionSet::empty(height, width); t_max + 1];
        let relevant_positions = vec![vec![PositionSet::empty(height, width); t_max + 1]; n_agents];
        let relevant_laser_paths =
            vec![vec![PositionSet::empty(height, width); t_max + 1]; laser_sources.len()];

        Self {
            t_max,
            n_agents,
            start_pos,
            predecessors,
            solution_lower_bound,
            laser_sources,
            exits,
            height,
            width,
            neighbours,
            updated_until: None,
            distance_buckets,
            exit_reachable,
            relevant_positions,
            forbidden_first_beam_tiles,
            relevant_laser_paths,
        }
    }

    /// Compute and cache the positions from which an exit is still reachable at time `t`.
    ///
    /// At `t = 0`, this seeds the cache with every position whose exit distance fits within the
    /// horizon.
    ///
    /// Later steps shrink the previous set by removing positions whose shortest exit
    /// distance no longer fits in the remaining time budget.
    fn update_exit_reachable(&mut self, t: usize) {
        if t == 0 {
            let mut result = PositionSet::empty(self.height, self.width);
            for bucket in &self.distance_buckets {
                result.union_with(bucket);
            }
            self.exit_reachable[0] = result;
            return;
        }

        let mut prev_exit_reachable = self.exit_reachable[t - 1].clone();
        let excluded_distance = self.t_max - t + 1;
        prev_exit_reachable.subtract_with(&self.distance_buckets[excluded_distance]);
        self.exit_reachable[t] = prev_exit_reachable;
    }

    /// Update the relevant positions for each agent at time step t.
    /// # Details
    /// A position is relevant to a given agent at time step t if:
    ///     - the agent can reach it at time step t
    ///     - the agent can still access the exit within `t_max - t` steps
    fn update_relevant_positions(&mut self, t: usize) {
        for agent in 0..self.n_agents {
            let mut result = if t == 0 {
                PositionSet::singleton(self.height, self.width, self.start_pos[agent])
            } else {
                let mut reachable = PositionSet::empty(self.height, self.width);
                for pos in &self.relevant_positions[agent][t - 1] {
                    for n in &self.neighbours[pos.i][pos.j] {
                        reachable.insert(n);
                    }
                }
                reachable
            };
            result.intersect_with(&self.exit_reachable[t]);
            // At t=1 no agent can occupy another agent's t=0 start position.
            // The no-following-conflict rule forbids agent A from being at start_B at t=1
            // because B was there at t=0 (implies(-a_cur, -b_prev) ⇒ ¬A here when B was here).
            if t == 1 {
                for (other, &start) in self.start_pos.iter().enumerate() {
                    if other != agent {
                        result.remove(&start);
                    }
                }
            }
            // Non-owner agents can never stand on the first tile of another agent's beam.
            result.subtract_with(&self.forbidden_first_beam_tiles[agent]);
            self.relevant_positions[agent][t] = result;
        }
    }

    /// Update every laser-derived relevance cache for time `t`.
    ///
    /// This first removes foreign-colour agent positions on beam tiles that cannot be made safe by
    /// an upstream owner block, then computes the relevant laser paths from the pruned position
    /// sets. Keeping all pruning before all path computation avoids order-dependent results when
    /// laser paths overlap or cross.
    fn update_laser_relevance(&mut self, t: usize) {
        for source in &self.laser_sources {
            // We use `split_at_mut` to avoid cloning the owner positions while respecting ownership rules.
            let (before_owner, owner_and_after) =
                self.relevant_positions.split_at_mut(source.agent_id);
            let (owner_positions, after_owner) = owner_and_after
                .split_first_mut()
                .expect("laser owner index must refer to an existing agent");
            let owner_reachable = &owner_positions[t];

            let mut blockable_upstream = false;
            for &pos in &source.path {
                if !blockable_upstream {
                    for positions in before_owner.iter_mut().chain(after_owner.iter_mut()) {
                        positions[t].remove(&pos);
                    }
                }
                if owner_reachable.contains(&pos) {
                    blockable_upstream = true;
                }
            }
        }

        for laser_idx in 0..self.laser_sources.len() {
            self.relevant_laser_paths[laser_idx][t] = compute_relevant_laser_path(
                &self.laser_sources[laser_idx].path,
                &self.relevant_positions,
                t,
                self.laser_sources[laser_idx].agent_id,
                self.height,
                self.width,
            );
        }
    }

    /// Remove exit positions that are incompatible with uniquely forced exit assignments.
    ///
    /// When there are exactly as many exits as agents, every exit tile must be occupied in any
    /// satisfying objective state because all agents must end on distinct exits. If an exit is
    /// reachable by exactly one agent at a timestep, and that agent is not the sole candidate for
    /// any other exit, then the agent is forced to use that exit and cannot have previously stopped
    /// on another exit.
    ///
    /// @ai-generated
    fn update_forced_exit_relevance(&mut self, t: usize) {
        if self.exits.size() != self.n_agents {
            return;
        }

        let mut forced_exit_by_agent = vec![None; self.n_agents];
        let mut unique_exit_count_by_agent = vec![0; self.n_agents];

        for exit in &self.exits {
            let mut unique_agent = None;
            let mut n_reachable_agents = 0;
            for agent in 0..self.n_agents {
                if self.relevant_positions[agent][t].contains(&exit) {
                    unique_agent = Some(agent);
                    n_reachable_agents += 1;
                }
            }
            if n_reachable_agents == 1 {
                let agent = unique_agent.expect("unique reachable exit must have an agent");
                forced_exit_by_agent[agent] = Some(exit);
                unique_exit_count_by_agent[agent] += 1;
            }
        }

        for agent in 0..self.n_agents {
            if unique_exit_count_by_agent[agent] != 1 {
                continue;
            }
            let forced_exit = forced_exit_by_agent[agent]
                .expect("agent with one uniquely reachable exit must have a forced exit");
            for exit in &self.exits {
                if exit != forced_exit {
                    self.relevant_positions[agent][t].remove(&exit);
                }
            }
        }
    }

    /// Pre-compute and cache the reachable positions and relevant laser paths needed to generate
    /// constraints for time step `t`.
    pub fn update(&mut self, t: usize) {
        if self
            .updated_until
            .is_some_and(|updated_until| t <= updated_until)
        {
            return;
        }
        let start = self
            .updated_until
            .map_or(0, |updated_until| updated_until + 1);
        for tt in start..=t {
            self.update_exit_reachable(tt);
            self.update_relevant_positions(tt);
            self.update_laser_relevance(tt);
            self.update_forced_exit_relevance(tt);
        }
        self.updated_until = Some(t);
    }

    /// Relevant positions for a single agent at time `t`, i.e. positions that the agent
    /// can reach and from which it can still access the exit within due time.
    pub fn relevant_positions_for_agent(&self, agent: usize, t: usize) -> &PositionSet {
        &self.relevant_positions[agent][t]
    }

    /// Positions relevant to *all* the given agents exactly at time `t` (already filtered by
    /// exit-reachability), i.e. intersection of the positions relevant to each agent individually.
    ///
    /// See `relevant_positions_for_agent` for more details.
    #[cfg(test)]
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

    /// Lazily iterate positions the agent could have occupied at `t - 1` to reach `pos` at `t`.
    /// Assumes `update` has already been called for this `t` and that `t > 0`.
    ///
    /// @ai-generated
    pub fn prev_neighbours_iter(&self, agent: usize, pos: &Position, t: usize) -> Intersection<'_> {
        self.predecessors[pos.i][pos.j].intersection(&self.relevant_positions[agent][t - 1])
    }

    /// Materialize predecessor positions for tests that exercise the `t == 0` boundary.
    #[cfg(test)]
    pub fn prev_neighbours(&self, agent: usize, pos: &Position, t: usize) -> PositionSet {
        if t == 0 {
            return PositionSet::empty(self.height, self.width);
        }
        let mut predecessors = self.predecessors[pos.i][pos.j].clone();
        predecessors.intersect_with(&self.relevant_positions[agent][t - 1]);
        predecessors
    }

    /// The reachable laser tiles positions for a given laser source at time `t`: the beam tiles
    /// that can still be blocked. Assumes `update` has already been called for this `t`.
    pub fn relevant_laser_tiles(&self, laser_id: usize, t: usize) -> &PositionSet {
        &self.relevant_laser_paths[laser_id][t]
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

/// Compute laser tiles that are relevant to consider at time step t.
///
/// # Details
/// The relevant laser path for a given laser source at time `t` is the subset of its beam tiles
/// worth reasoning about. A tile is relevant if either:
///   - the owning agent can reach it at time `t` (it can block the beam there), or
///   - it lies downstream of such a blockable tile *and* some other agent can reach it at time
///     `t`. Because the owner could block the beam upstream, the tile may become safe for that
///     other agent — so it is a tile where cooperation matters.
///
/// # Assumptions
/// This function assumes that `update_relevant_positions` has already been called for `t`.
/// Compute the relevant beam tiles for one laser source at time `t`.
///
fn compute_relevant_laser_path(
    path: &[Position],
    relevant_positions: &[Vec<PositionSet>],
    t: usize,
    owner_id: usize,
    height: usize,
    width: usize,
) -> PositionSet {
    let n_agents = relevant_positions.len();
    let owner_reachable = &relevant_positions[owner_id][t];
    let mut result = PositionSet::empty(height, width);
    // Whether an upstream tile can be blocked by the owner: once true, every downstream tile can
    // be made safe by blocking the beam upstream.
    let mut blockable_upstream = false;
    for &pos in path {
        if owner_reachable.contains(&pos) {
            result.insert(pos);
            blockable_upstream = true;
        } else if blockable_upstream
            && (0..n_agents).any(|a| a != owner_id && relevant_positions[a][t].contains(&pos))
        {
            result.insert(pos);
        }
    }
    result
}

fn compute_exit_distance(
    exits: &PositionSet,
    predecessors: &[Vec<PositionSet>],
) -> HashMap<Position, usize> {
    let mut dist: HashMap<Position, usize> = exits.iter().map(|p| (p, 0)).collect();
    let mut frontier: VecDeque<Position> = exits.iter().collect();
    while let Some(current) = frontier.pop_front() {
        let current_dist = dist[&current];
        for pred in &predecessors[current.i][current.j] {
            dist.entry(pred).or_insert_with(|| {
                frontier.push_back(pred);
                current_dist + 1
            });
        }
    }
    dist
}

#[cfg(test)]
#[path = "../unit_tests/test_context.rs"]
mod tests;
