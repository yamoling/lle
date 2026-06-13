use std::collections::HashSet;

use crate::solver::errors::SolverError;
use crate::{Action, AgentId, Position, World};

use super::super::context::ConstraintContext;
use super::Clause;
use super::Literal;
use super::{VarKey, VarPool};

/// Determines which extra clauses/assumptions `ClauseGenerator::generate` emits.
#[derive(Clone, Copy, Default)]
pub enum SolveMode {
    /// Standard world rules only.
    #[default]
    Standard,
    /// No non-owner agent may enter any laser span.
    NoCooperation,
    /// No pair of agents may mutually cooperate (each helping the other).
    NoMutualCooperation,
    /// No temporal chain `a → b → c` (a helped b, then b helped c) may appear; also rules out
    /// mutual cycles. This is strictly stronger than `NoMutualCooperation`.
    NoChainedCooperation,
    /// No temporal cycle of order ≥ `k` may appear in any solution. A temporal cycle of order `k`
    /// visits `k` distinct agents and closes back to the start, with non-decreasing timestamps.
    NoInterdependence { k: usize },
}

impl SolveMode {
    pub fn from_str(s: &str) -> Result<Self, String> {
        match s {
            "standard" => Ok(SolveMode::Standard),
            "no-cooperation" => Ok(SolveMode::NoCooperation),
            "no-mutual-cooperation" => Ok(SolveMode::NoMutualCooperation),
            "no-chained-cooperation" => Ok(SolveMode::NoChainedCooperation),
            s if s.starts_with("no-interdependence-") => {
                let suffix = &s["no-interdependence-".len()..];
                let k = suffix.parse::<usize>().map_err(|_| {
                    format!("Invalid k in mode '{s}'. Expected 'no-interdependence-N' where N is a positive integer.")
                })?;
                if k < 2 {
                    return Err(format!("Interdependence order k must be ≥ 2, got {k}."));
                }
                Ok(SolveMode::NoInterdependence { k })
            }
            _ => Err(format!(
                "Unknown solve mode: '{s}'. Expected one of: 'standard', 'no-cooperation', \
                 'no-mutual-cooperation', 'no-chained-cooperation', 'no-interdependence-N'."
            )),
        }
    }
}

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

/// Generates the SAT clauses for a bounded planning horizon, combining initialization,
/// movement, laser constraints, mode-specific constraints, and the objective.
pub struct ClauseGenerator {
    pub(super) ctx: ConstraintContext,
    pub(super) pool: VarPool,
    pub(super) exits: HashSet<Position>,
    mode: SolveMode,
    /// Simple directed cycles considered when mode is `NoInterdependence`.
    /// Each entry is a list of agent ids [v0, v1, ..., v_{k-1}] where v0 is the smallest.
    pub(super) interdep_cycles: Vec<Vec<AgentId>>,
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
        let interdep_cycles = if let SolveMode::NoInterdependence { k } = mode {
            let mut agents: Vec<AgentId> =
                ctx.laser_sources.iter().map(|s| s.agent_id).collect();
            agents.sort_unstable();
            agents.dedup();
            enumerate_directed_cycles(&agents, k)
        } else {
            vec![]
        };
        Self {
            exits: world.exits_positions().into_iter().collect(),
            ctx,
            pool: VarPool::new(),
            mode,
            interdep_cycles,
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
    /// - For `NoMutualCooperation`: the current mutual-forbid clauses and assumptions
    /// - For `NoCooperation`: per-step no-cooperation assumptions for steps `0..=t`
    pub fn generate(&mut self, t: usize) -> (Vec<Clause>, Vec<Literal>) {
        let start = self.generated_until.map_or(0, |u| u + 1);
        for tt in start..=t {
            self.ctx.update(tt);
            self.fill_clauses(tt);
            self.fill_assumptions(tt);
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
            SolveMode::NoMutualCooperation => {
                let (mc, ma) = self.forbid_mutual_cooperation();
                clauses.extend(mc);
                assumptions.extend(ma);
            }
            SolveMode::NoChainedCooperation => {
                let (cc, ca) = self.forbid_chained_cooperation();
                clauses.extend(cc);
                assumptions.extend(ca);
            }
            SolveMode::NoInterdependence { .. } => {
                let (ic, ia) = self.forbid_interdependence();
                clauses.extend(ic);
                assumptions.extend(ia);
            }
            _ => {}
        }

        (clauses, assumptions)
    }

    fn fill_clauses(&mut self, t: usize) {
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
            SolveMode::NoMutualCooperation => {
                clauses.extend(self.dependency_clauses(t));
            }
            SolveMode::NoChainedCooperation => {
                clauses.extend(self.dependency_clauses(t));
                clauses.extend(self.chain_clauses(t));
            }
            SolveMode::NoInterdependence { .. } => {
                clauses.extend(self.interdependence_clauses(t));
            }
            _ => {}
        }
        self.clause_buffer[t] = clauses;
    }

    fn fill_assumptions(&mut self, t: usize) {
        self.assumption_buffer[t] = match self.mode {
            SolveMode::Standard
            | SolveMode::NoMutualCooperation
            | SolveMode::NoChainedCooperation
            | SolveMode::NoInterdependence { .. } => vec![],
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
