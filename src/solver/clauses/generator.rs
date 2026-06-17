use std::collections::HashSet;

use crate::solver::errors::SolverError;
use crate::{Action, AgentId, Position, World};

use super::super::context::ConstraintContext;
use super::Clause;
use super::Literal;
use super::solve_mode::SolveMode;
use super::{VarKey, VarPool};

/// Enumerate all simple directed cycles of order ≥ `min_order` over `agents`.
///
/// Each cycle is returned as a Vec whose first element is the lexicographically-smallest agent
/// in the cycle (canonical form that avoids counting the same cycle under different rotations).
pub(crate) fn enumerate_directed_cycles(agents: &[AgentId], min_order: usize) -> Vec<Vec<AgentId>> {
    let mut cycles = Vec::new();
    for (start_idx, &start) in agents.iter().enumerate() {
        let available: Vec<AgentId> = agents[start_idx + 1..].to_vec();
        cycles_dfs(start, &available, vec![start], min_order, &mut cycles);
    }
    cycles
}

fn cycles_dfs(
    root: AgentId,
    available: &[AgentId],
    path: Vec<AgentId>,
    min_order: usize,
    out: &mut Vec<Vec<AgentId>>,
) {
    if path.len() >= min_order {
        out.push(path.clone());
    }
    for (i, &next) in available.iter().enumerate() {
        let mut new_avail = available.to_vec();
        new_avail.remove(i);
        let mut new_path = path.clone();
        new_path.push(next);
        cycles_dfs(root, &new_avail, new_path, min_order, out);
    }
}

/// Enumerate all directed trails of exactly `length` edges starting from a laser owner.
///
/// Each trail is returned as `[v0, v1, …, v_length]` where:
/// - `v0, …, v_{length-1}` must be laser owners (they are helpers);
/// - `v_length` can be any agent (the final beneficiary);
/// - no directed pair `(vi, v_{i+1})` appears twice (trail condition);
/// - no self-loops (`vi ≠ v_{i+1}`).
pub(crate) fn enumerate_directed_trails(
    owners: &[AgentId],
    all_agents: &[AgentId],
    length: usize,
) -> Vec<Vec<AgentId>> {
    if length < 2 {
        return vec![];
    }
    let mut out = Vec::new();
    let mut edges_used = HashSet::new();
    for &start in owners {
        let mut path = vec![start];
        trail_dfs(start, owners, all_agents, &mut edges_used, &mut path, length, &mut out);
    }
    out
}

fn trail_dfs(
    current: AgentId,
    owners: &[AgentId],
    all_agents: &[AgentId],
    edges_used: &mut HashSet<(AgentId, AgentId)>,
    path: &mut Vec<AgentId>,
    target_edges: usize,
    out: &mut Vec<Vec<AgentId>>,
) {
    let current_edges = path.len() - 1;
    if current_edges == target_edges {
        out.push(path.clone());
        return;
    }
    let remaining = target_edges - current_edges;
    // All helpers except the last must be laser owners; the final beneficiary can be any agent.
    let candidates: &[AgentId] = if remaining == 1 { all_agents } else { owners };
    for &next in candidates {
        if next == current {
            continue;
        }
        let edge = (current, next);
        if !edges_used.contains(&edge) {
            edges_used.insert(edge);
            path.push(next);
            trail_dfs(next, owners, all_agents, edges_used, path, target_edges, out);
            path.pop();
            edges_used.remove(&edge);
        }
    }
}

/// Generates the SAT clauses for a bounded planning horizon, combining initialization,
/// movement, laser constraints, mode-specific constraints, and the objective.
pub struct ClauseGenerator {
    pub(super) ctx: ConstraintContext,
    pub(super) pool: VarPool,
    pub(super) exits: HashSet<Position>,
    pub(super) mode: SolveMode,
    /// Directed trails/cycles detected by walk-progress tracking. Each entry is a vertex sequence
    /// `[v0, …, v_m]` (edge `i` is `v_i → v_{i+1}`).  For `NoInterdependence`, entries are
    /// closed cycles (`v_m == v_0`). For `NoChainedCooperation`, entries are open trails with no
    /// repeated directed pairs.
    pub(super) walks: Vec<Vec<AgentId>>,
    /// Ordered `(helper, beneficiary)` pairs for which a `has_helped_by_time` indicator is
    /// actually consumed — all owner-to-agent edges for asymmetric mode, mutual owner-pairs for
    /// mutual mode, or each walk's first edge for interdependence and chain modes.
    pub(super) has_helped_pairs: Vec<(AgentId, AgentId)>,
    /// `clause_buffer[t]` = world-enforcing (+ mode-specific) clauses for step `t`.
    clause_buffer: Vec<Vec<Clause>>,
    /// `assumption_buffer[t]` = per-step assumptions for step `t`.
    assumption_buffer: Vec<Vec<Literal>>,
    /// Steps 0..=generated_until have been buffered; `None` means nothing buffered yet.
    generated_until: Option<usize>,
}

impl ClauseGenerator {
    pub fn new(world: &World, t_max: usize, mode: SolveMode) -> Self {
        let ctx = ConstraintContext::new(world, t_max);
        // Agents that own a laser are the only ones that can ever help (the helper of every walk
        // edge must block a beam).
        let mut owners: Vec<AgentId> = ctx.laser_sources.iter().map(|s| s.agent_id).collect();
        owners.sort_unstable();
        owners.dedup();
        let all_agents: Vec<AgentId> = (0..ctx.n_agents).collect();
        let walks = match mode {
            // Any simple directed cycle of order ≥ `n`, expanded to a closed vertex sequence.
            SolveMode::NoInterdependence(n) => enumerate_directed_cycles(&owners, n)
                .into_iter()
                .map(|mut cycle| {
                    cycle.push(cycle[0]);
                    cycle
                })
                .collect(),
            // All directed trails of exactly `k` edges; forbidding each one prevents chains ≥ k
            // because every longer trail contains a sub-trail of length k.
            SolveMode::NoChainedCooperation(k) => {
                enumerate_directed_trails(&owners, &all_agents, k)
            }
            _ => vec![],
        };
        // `has_helped_by_time` is generated only for directed pairs consumed by the selected
        // cooperation mode: all owner-to-agent edges for asymmetric mode, mutual owner-pairs for
        // mutual mode, or each walk's first edge for interdependence and chain modes.
        let has_helped_pairs: Vec<(AgentId, AgentId)> = match mode {
            SolveMode::NoAsymmetricCooperation => owners
                .iter()
                .flat_map(|&helper| {
                    (0..ctx.n_agents)
                        .filter(move |&beneficiary| beneficiary != helper)
                        .map(move |beneficiary| (helper, beneficiary))
                })
                .collect(),
            SolveMode::NoMutualCooperation => owners
                .iter()
                .flat_map(|&a| {
                    owners
                        .iter()
                        .filter(move |&&b| b != a)
                        .map(move |&b| (a, b))
                })
                .collect(),
            SolveMode::NoInterdependence(_) | SolveMode::NoChainedCooperation(_) => {
                let mut pairs: Vec<(AgentId, AgentId)> =
                    walks.iter().map(|w| (w[0], w[1])).collect();
                pairs.sort_unstable();
                pairs.dedup();
                pairs
            }
            _ => vec![],
        };
        Self {
            exits: world.exits_positions().into_iter().collect(),
            ctx,
            pool: VarPool::new(),
            mode,
            walks,
            has_helped_pairs,
            clause_buffer: vec![Vec::new(); t_max + 1],
            assumption_buffer: vec![Vec::new(); t_max + 1],
            generated_until: None,
        }
    }

    /// Generate all clauses and assumptions required to solve the problem at step `t`.
    ///
    /// Fills the internal buffers for any steps not yet cached, then returns:
    /// - All buffered world-enforcing (and mode-specific) clauses for steps `0..=t`
    /// - The objective clauses for horizon `t` (every agent on an exit)
    /// - For `NoAsymmetricCooperation`: the current asymmetric-forbid clauses and assumptions
    /// - For `NoMutualCooperation`: the current mutual-forbid clauses and assumptions
    /// - For `NoCooperation`: per-step no-cooperation assumptions for steps `0..=t`
    pub fn generate(&mut self, t: usize) -> (Vec<Clause>, Vec<Literal>) {
        let start = self.generated_until.map_or(0, |u| u + 1);
        for tt in start..=t {
            self.ctx.update(tt);
            self.generate_clauses(tt);
            self.generate_assumptions(tt);
        }
        if start <= t {
            self.generated_until = Some(t);
        }

        let mut clauses: Vec<Clause> = self.clause_buffer[..=t].iter().flatten().cloned().collect();
        let mut assumptions: Vec<Literal> = self.assumption_buffer[..=t]
            .iter()
            .flatten()
            .copied()
            .collect();
        clauses.extend(self.objective(t));
        match self.mode {
            SolveMode::NoAsymmetricCooperation => {
                // Asymmetric-forbid clauses depend on the current solve horizon `t` because an
                // earlier help event is non-asymmetric if the helper is helped at any later step
                // up to `t`. Do not cache these in `fill_clauses`; see
                // `forbid_asymmetric_cooperation` for details.
                let (ac, aa) = self.forbid_asymmetric_cooperation(t);
                clauses.extend(ac);
                assumptions.extend(aa);
            }
            SolveMode::NoMutualCooperation => {
                let (mc, ma) = self.forbid_mutual_cooperation(t);
                clauses.extend(mc);
                assumptions.extend(ma);
            }
            SolveMode::NoChainedCooperation(_) | SolveMode::NoInterdependence(_) => {
                let (wc, wa) = self.forbid_walks();
                clauses.extend(wc);
                assumptions.extend(wa);
            }
            _ => {}
        }

        (clauses, assumptions)
    }

    fn generate_clauses(&mut self, t: usize) {
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
        match self.mode {
            SolveMode::NoAsymmetricCooperation
            | SolveMode::NoMutualCooperation
            | SolveMode::NoInterdependence(_)
            | SolveMode::NoChainedCooperation(_) => clauses.extend(self.has_helped_by_time_clauses(t)),
            _ => {}
        }
        match self.mode {
            SolveMode::NoChainedCooperation(_) | SolveMode::NoInterdependence(_) => {
                clauses.extend(self.walk_clauses(t))
            }
            _ => {}
        }
        self.clause_buffer[t] = clauses;
    }

    fn generate_assumptions(&mut self, t: usize) {
        self.assumption_buffer[t] = match self.mode {
            SolveMode::Standard
            | SolveMode::NoAsymmetricCooperation
            | SolveMode::NoMutualCooperation
            | SolveMode::NoChainedCooperation(_)
            | SolveMode::NoInterdependence(_) => vec![],
            SolveMode::NoCooperation => self.assume_no_cooperation(t),
        };
    }

    /// Objective clauses for horizon `t`: every agent must be on an exit. Not cached.
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
                    .map(|p| self.pool.agent(agent, p, t))
                    .collect(),
            );
        }
        clauses
    }

    #[inline]
    pub fn decode_plan(
        &self,
        literals: &[i32],
        t_end: usize,
    ) -> Result<Vec<Vec<Action>>, SolverError> {
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

    pub fn exists(&self, key: &VarKey) -> bool {
        self.pool.exists(key)
    }

    pub fn n_vars(&self) -> usize {
        self.pool.n_vars()
    }

    /// Return the SAT literal assigned to `key`, or `None` if it was never created.
    /// Useful in tests to inspect clause literals without accessing the pool directly.
    pub fn literal(&self, key: &VarKey) -> Option<i32> {
        self.pool.get(key)
    }
}
