use std::collections::HashMap;

use itertools::Itertools;

use crate::solver::VarKey;
use crate::tiles::LaserId;
use crate::{Action, AgentId, Position, World};

use super::context::ConstraintContext;
use super::var_pool::VarPool;

pub type Clause = Vec<i32>;

/// At-most-one encoding crossover: for small variable sets, the naive pairwise encoding
/// (n(n-1)/2 binary clauses, no auxiliary variables) uses fewer-or-equal clauses *and* zero
/// auxiliary variables compared to a sequential-counter encoding, which only wins on clause
/// count from n >= 6 onward (at the cost of n-1 auxiliary variables).
const PAIRWISE_ATMOST_MAX: usize = 5;

fn implies(a: i32, b: i32) -> Clause {
    vec![-a, b]
}

fn equals(a: i32, b: i32) -> Vec<Clause> {
    vec![implies(a, b), implies(b, a)]
}

/// Sequential-counter at-most-one encoding (mirrors `pysat.card.CardEnc.atmost(bound=1)`),
/// used once pairwise encoding stops being competitive.
fn at_most_one_sequential(vars: &[i32], pool: &mut VarPool) -> Vec<Clause> {
    // Sequential-counter encoding: introduce auxiliary variables s_i meaning
    // "at least one of vars[0..=i] is true", forbidding any later variable once one is set.
    let n = vars.len();
    let mut s = Vec::with_capacity(n - 1);
    for _ in 0..n - 1 {
        s.push(pool.aux());
    }
    let mut clauses = Vec::new();
    for i in 0..n - 1 {
        clauses.push(implies(vars[i], s[i])); // vars[i] -> s[i]
        clauses.push(vec![-vars[i + 1], -s[i]]); // vars[i+1] -> ¬s[i]
        if i + 1 < n - 1 {
            clauses.push(implies(s[i], s[i + 1])); // s[i] -> s[i+1]
        }
    }
    clauses
}

/// Generates the SAT clauses for a single time step `t`, mirroring
/// `InitializationConstraints`, `MovementConstraints` and `LaserConstraints` combined.
pub struct ClauseGenerator {
    ctx: ConstraintContext,
    pool: VarPool,
    exits: std::collections::HashSet<Position>,
}

impl ClauseGenerator {
    pub fn new(world: &World, t_max: usize) -> Self {
        Self {
            exits: world.exits_positions().into_iter().collect(),
            ctx: ConstraintContext::new(world, t_max),
            pool: VarPool::new(),
        }
    }

    #[inline]
    pub fn decode_plan(&self, literals: &[i32], t_end: usize) -> Result<Vec<Vec<Action>>, String> {
        self.pool.decode_plan(literals, t_end)
    }

    #[inline]
    pub fn t_max(&self) -> usize {
        self.ctx.t_max
    }

    #[inline]
    pub fn solution_lower_bound(&self) -> usize {
        self.ctx.solution_lower_bound
    }

    #[inline]
    fn agent(&mut self, agent: AgentId, pos: Position, t: usize) -> i32 {
        self.pool.agent(agent, pos, t)
    }

    #[inline]
    fn laser(&mut self, laser_id: LaserId, pos: Position, t: usize) -> i32 {
        self.pool.laser(laser_id, pos, t)
    }

    pub fn exists(&self, key: &VarKey) -> bool {
        self.pool.exists(&key)
    }

    /// All clauses for time step `t`: initialization (t == 0), movement and laser constraints.
    pub fn generate(&mut self, t: usize) -> Vec<Clause> {
        self.ctx.update(t);
        let mut clauses = Vec::new();
        clauses.extend(self.initialization(t));
        clauses.extend(self.exactly_one_position(t));
        clauses.extend(self.time_wise_adjacency(t));
        clauses.extend(self.no_overlap(t));
        clauses.extend(self.no_following_conflict(t));
        clauses.extend(self.stays_on_exit(t));
        let (beam_clauses, active_lit) = self.beam_activation(t);
        clauses.extend(beam_clauses);
        clauses.extend(self.no_step_on_active_laser(t, &active_lit));
        clauses
    }

    /// Clauses asserting that, at the final time step `t`, every agent is on an exit.
    pub fn objective(&mut self, t: usize) -> Vec<Clause> {
        self.ctx.update(t);
        let mut clauses = Vec::with_capacity(self.ctx.n_agents);
        for agent in 0..self.ctx.n_agents {
            let reachable = self.ctx.relevant_positions(t, &[agent]);
            let positions: Vec<Position> = self
                .exits
                .iter()
                .copied()
                .filter(|p| reachable.contains(p))
                .collect();
            clauses.push(
                positions
                    .into_iter()
                    .map(|p| self.agent(agent, p, t))
                    .collect(),
            );
        }
        clauses
    }

    /// Unit clauses forbidding any laser-blocking event at time `t` (used by `solve_no_cooperation`,
    /// logically equivalent to a strict laser mode where cooperation is impossible).
    pub fn no_blocking_clauses(&mut self, t: usize) -> Vec<Clause> {
        self.ctx.update(t);
        let mut clauses = Vec::new();
        for idx in 0..self.ctx.laser_sources.len() {
            let agent_id = self.ctx.laser_sources[idx].agent_id;
            let positions = self.ctx.get_reachable_laser_path(idx, t).clone();
            for pos in positions {
                let agent_var = self.agent(agent_id, pos, t);
                clauses.push(vec![-agent_var]);
            }
        }
        clauses
    }

    // ------------------------------------------------------------------
    // Initialization
    // ------------------------------------------------------------------

    fn initialization(&mut self, t: usize) -> Vec<Clause> {
        if t != 0 {
            return Vec::new();
        }
        let starts = self.ctx.start_pos.clone();
        starts
            .into_iter()
            .enumerate()
            .map(|(agent, pos)| vec![self.agent(agent, pos, 0)])
            .collect()
    }

    // ------------------------------------------------------------------
    // Movement
    // ------------------------------------------------------------------

    /// Every agent is in exactly one position at any given time step.
    fn exactly_one_position(&mut self, t: usize) -> Vec<Clause> {
        let mut clauses = Vec::new();
        for agent in 0..self.ctx.n_agents {
            let positions: Vec<Position> = self
                .ctx
                .relevant_positions(t, &[agent])
                .into_iter()
                .collect();
            if positions.len() <= 1 {
                continue;
            }
            let vars: Vec<i32> = positions
                .into_iter()
                .map(|p| self.agent(agent, p, t))
                .collect();
            clauses.push(vars.clone());
            if vars.len() <= PAIRWISE_ATMOST_MAX {
                for i in 0..vars.len() {
                    for j in i + 1..vars.len() {
                        clauses.push(vec![-vars[i], -vars[j]]);
                    }
                }
            } else {
                clauses.extend(at_most_one_sequential(&vars, &mut self.pool));
            }
        }
        clauses
    }

    /// If an agent is at `(x, y)` at time `t`, it must have been in an adjacent cell at `t - 1`.
    fn time_wise_adjacency(&mut self, t: usize) -> Vec<Clause> {
        if t == 0 {
            return Vec::new();
        }
        let mut clauses = Vec::new();
        for agent in 0..self.ctx.n_agents {
            let positions: Vec<Position> = self
                .ctx
                .relevant_positions(t, &[agent])
                .into_iter()
                .collect();
            for pos in positions {
                let prev_positions = self.ctx.prev_neighbours(agent, &pos, t);
                let current_var = self.agent(agent, pos, t);
                let mut clause = vec![-current_var];
                for prev in prev_positions {
                    clause.push(self.agent(agent, prev, t - 1));
                }
                clauses.push(clause);
            }
        }
        clauses
    }

    /// Two agents cannot occupy the same cell at the same time.
    fn no_overlap(&mut self, t: usize) -> Vec<Clause> {
        let mut clauses = Vec::new();
        for c1 in 0..self.ctx.n_agents {
            for c2 in c1 + 1..self.ctx.n_agents {
                let positions: Vec<Position> = self
                    .ctx
                    .relevant_positions(t, &[c1, c2])
                    .into_iter()
                    .collect();
                for pos in positions {
                    let v1 = self.agent(c1, pos, t);
                    let v2 = self.agent(c2, pos, t);
                    clauses.push(vec![-v1, -v2]);
                }
            }
        }
        clauses
    }

    /// Prevent two agents from swapping positions (vertex-following conflicts).
    fn no_following_conflict(&mut self, t: usize) -> Vec<Clause> {
        if t == 0 || self.ctx.n_agents == 0 {
            return Vec::new();
        }
        let mut clauses = Vec::new();
        for (c1, c2) in (0..self.ctx.n_agents).tuple_combinations() {
            let prev_c1 = self.ctx.relevant_positions(t - 1, &[c1]);
            let cur_c2 = self.ctx.relevant_positions(t, &[c2]);
            for pos in prev_c1.intersection(&cur_c2) {
                let a2 = self.agent(c2, pos, t);
                let a1_prev = self.agent(c1, pos, t - 1);
                clauses.push(implies(a2, -a1_prev));
            }
            let cur_c1 = self.ctx.relevant_positions(t, &[c1]);
            let prev_c2 = self.ctx.relevant_positions(t - 1, &[c2]);
            for pos in cur_c1.intersection(&prev_c2) {
                let a1 = self.agent(c1, pos, t);
                let a2_prev = self.agent(c2, pos, t - 1);
                clauses.push(implies(a1, -a2_prev));
            }
        }
        clauses
    }

    /// If an agent was on an exit at `t - 1`, it must remain on an exit at `t`.
    fn stays_on_exit(&mut self, t: usize) -> Vec<Clause> {
        if t == 0 {
            return Vec::new();
        }
        let mut clauses = Vec::new();
        for agent in 0..self.ctx.n_agents {
            let reachable = self.ctx.relevant_positions(t - 1, &[agent]);
            let exit_positions: Vec<Position> = self
                .exits
                .iter()
                .copied()
                .filter(|p| reachable.contains(p))
                .collect();
            for pos in exit_positions {
                let prev = self.agent(agent, pos, t - 1);
                let cur = self.agent(agent, pos, t);
                clauses.push(vec![-prev, cur]);
            }
        }
        clauses
    }

    // ------------------------------------------------------------------
    // Lasers
    // ------------------------------------------------------------------

    /// Defines, for each beam tile, the literal denoting "this beam tile is active at time `t`",
    /// folding away tiles that no same-colour agent can ever reach (constant-active tiles).
    /// Returns both the clauses and a map from `(laser_id, x, y)` to the literal representing
    /// beam-tile activation; tiles absent from the map are constant-active.
    fn beam_activation(&mut self, t: usize) -> (Vec<Clause>, HashMap<(usize, usize, usize), i32>) {
        let mut clauses = Vec::new();
        let mut active_lit = HashMap::new();
        let sources: Vec<(usize, usize, Vec<Position>)> = self
            .ctx
            .laser_sources
            .iter()
            .map(|s| (s.agent_id, s.laser_id, s.path.clone()))
            .collect();
        for (agent_id, laser_id, path) in sources {
            let blockable = self.ctx.relevant_positions(t, &[agent_id]);
            let mut prev_active: Option<i32> = None;
            for pos in path {
                if blockable.contains(&pos) {
                    let agent_var = self.agent(agent_id, pos, t);
                    let active = self.laser(laser_id, pos, t);
                    match prev_active {
                        None => clauses.extend(equals(active, -agent_var)),
                        Some(prev) => {
                            clauses.push(implies(active, prev));
                            clauses.push(implies(active, -agent_var));
                            clauses.push(vec![-prev, agent_var, active]);
                        }
                    }
                    prev_active = Some(active);
                    active_lit.insert((laser_id, pos.i, pos.j), active);
                } else if let Some(prev) = prev_active {
                    active_lit.insert((laser_id, pos.i, pos.j), prev);
                }
                // else: constant-active tile, no variable, no clause.
            }
        }
        (clauses, active_lit)
    }

    /// Agents cannot step on an active laser beam of another colour.
    fn no_step_on_active_laser(
        &mut self,
        t: usize,
        active_lit: &HashMap<(usize, usize, usize), i32>,
    ) -> Vec<Clause> {
        let mut clauses = Vec::new();
        let sources: Vec<(usize, usize, Vec<Position>)> = self
            .ctx
            .laser_sources
            .iter()
            .map(|s| (s.agent_id, s.laser_id, s.path.clone()))
            .collect();
        for agent in 0..self.ctx.n_agents {
            let reachable = self.ctx.relevant_positions(t, &[agent]);
            for &(source_agent_id, laser_id, ref path) in &sources {
                if source_agent_id == agent {
                    continue;
                }
                for &pos in path {
                    if !reachable.contains(&pos) {
                        continue;
                    }
                    let agent_var = self.agent(agent, pos, t);
                    match active_lit.get(&(laser_id, pos.i, pos.j)) {
                        Some(&lit) => clauses.push(vec![-agent_var, -lit]),
                        None => clauses.push(vec![-agent_var]), // constant-active beam tile
                    }
                }
            }
        }
        clauses
    }
}

#[cfg(test)]
#[path = "../unit_tests/test_individual_clauses.rs"]
mod tests;
