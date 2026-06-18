use std::collections::HashSet;

use itertools::Itertools;

use crate::solver::errors::SolverError;
use crate::{Action, AgentId, Position, World};

use super::super::context::ConstraintContext;
use super::Clause;
use super::Literal;
use super::solve_mode::SolveMode;
use super::trails::enumerate_for_mode;
use super::{VarKey, VarPool};

/// Generates the SAT clauses for a bounded planning horizon, combining initialization,
/// movement, laser constraints, mode-specific constraints, and the objective.
pub struct ClauseGenerator {
    pub(super) ctx: ConstraintContext,
    pub(super) pool: VarPool,
    pub(super) exits: HashSet<Position>,
    pub(super) gems: Vec<Position>,
    pub(super) mode: SolveMode,
    /// Whether gems should be collected in the objective function.
    collect_gems: bool,
    /// Directed trails/cycles detected by trail-progress tracking. Each entry is a vertex sequence
    /// `[v0, …, v_m]` (edge `i` is `v_i → v_{i+1}`).  For `NoInterdependence`, entries are
    /// closed cycles (`v_m == v_0`). For `NoChainedCooperation`, entries are open trails with no
    /// repeated directed pairs.
    pub(super) trails: Vec<Vec<AgentId>>,
    /// Ordered `(helper, beneficiary)` pairs for which a `has_helped_by_time` indicator is
    /// actually consumed — all owner-to-agent edges for asymmetric mode, mutual owner-pairs for
    /// mutual mode, or each trail's first edge for interdependence and chain modes.
    pub(super) has_helped_pairs: Vec<(AgentId, AgentId)>,
    /// `clause_buffer[t]` = world-enforcing (+ mode-specific) clauses for step `t`.
    clause_buffer: Vec<Vec<Clause>>,
    /// `assumption_buffer[t]` = per-step assumptions for step `t`.
    assumption_buffer: Vec<Vec<Literal>>,
    /// Steps 0..=generated_until have been buffered; `None` means nothing buffered yet.
    generated_until: Option<usize>,
}

impl ClauseGenerator {
    pub fn new(world: &World, t_max: usize, mode: SolveMode, collect_gems: bool) -> Self {
        let ctx = ConstraintContext::new(world, t_max);
        // Agents that own a laser are the only ones that can ever help (the helper of every trail
        // edge must block a beam).
        let owners: Vec<AgentId> = ctx
            .laser_sources
            .iter()
            .map(|s| s.agent_id)
            .unique()
            .collect();
        let all_agents: Vec<AgentId> = (0..ctx.n_agents).collect();
        let trails = enumerate_for_mode(mode, &owners, &all_agents);
        // `has_helped_by_time` is generated only for directed pairs consumed by the selected
        // cooperation mode: all owner-to-agent edges for asymmetric mode, mutual owner-pairs for
        // mutual mode, or each trail's first edge for interdependence and chain modes.
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
                    trails.iter().map(|w| (w[0], w[1])).collect();
                pairs.sort_unstable();
                pairs.dedup();
                pairs
            }
            _ => vec![],
        };
        Self {
            exits: world.exits_positions().into_iter().collect(),
            gems: world.gems_positions(),
            ctx,
            pool: VarPool::new(),
            mode,
            trails,
            collect_gems,
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
        let (forbid_clauses, forbid_assumptions) = self.forbid_cooperation(t);
        clauses.extend(forbid_clauses);
        assumptions.extend(forbid_assumptions);

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
        if self.mode.needs_has_helped() {
            clauses.extend(self.has_helped_by_time_clauses(t));
        }
        if self.mode.uses_trails() {
            clauses.extend(self.trail_clauses(t));
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
        let mut clauses = if self.collect_gems {
            self.gems_must_be_collected(t)
        } else {
            Vec::with_capacity(self.ctx.n_agents)
        };
        for agent in 0..self.ctx.n_agents {
            let reachable = self.ctx.relevant_positions_for_agent(agent, t);
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
