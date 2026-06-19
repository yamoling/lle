use super::generator::ClauseGenerator;
use super::solve_mode::SolveMode;
use super::utils::implies;
use super::{Clause, Literal, VarKey};
use crate::{AgentId, Position};

#[derive(Clone, Copy)]
enum DependencyKind {
    Chain { length: usize },
    Cycle { order: usize },
}

impl ClauseGenerator {
    /// Horizon-specific forbid clauses/assumptions for `mode`.
    pub(crate) fn mode_forbid(&mut self, t: usize, mode: SolveMode) -> (Vec<Clause>, Vec<Literal>) {
        match mode {
            SolveMode::Standard => (vec![], vec![]),
            SolveMode::NoCooperation => (vec![], self.assume_no_cooperation_until(t)),
            SolveMode::NoAsymmetricCooperation => self.forbid_asymmetric_cooperation(t),
            SolveMode::NoMutualCooperation => self.forbid_mutual_cooperation(t),
            SolveMode::NoChainedCooperation(length) => self.forbid_chains(length),
            SolveMode::NoInterdependence(order) => self.forbid_cycle_rotations(order),
        }
    }

    /// Positions where `beneficiary` can legally stand on one of `helper`'s beams at time `t`.
    ///
    /// Such an occupancy is a *help edge* `helper → beneficiary`: [`no_step_on_active_laser`]
    /// forbids any non-owner from standing on an *active* beam tile, so the only way `beneficiary`
    /// can be there is if `helper` blocks the beam upstream. This single primitive replaces the
    /// `beam ∩ reachable` intersection that every cooperation-aware mode used to open-code.
    ///
    /// [`no_step_on_active_laser`]: Self::no_step_on_active_laser
    fn help_edge_positions(
        &self,
        helper: AgentId,
        beneficiary: AgentId,
        t: usize,
    ) -> Vec<Position> {
        let reachable = self.ctx.relevant_positions_for_agent(beneficiary, t);
        self.ctx
            .laser_sources
            .iter()
            .filter(|s| s.agent_id == helper)
            .flat_map(|s| {
                self.ctx
                    .relevant_laser_tiles(s.laser_id, t)
                    .intersection(reachable)
            })
            .collect()
    }

    /// Clauses defining `has_helped_by_time(helper, beneficiary, t)` for every tracked help pair at
    /// time step `t`. This is a **monotone temporal prefix-OR**: it becomes true at the first help
    /// event and stays true forever after.
    ///
    /// ```text
    /// agent(beneficiary, q, t)                    → has_helped_by_time(helper, beneficiary, t)
    /// has_helped_by_time(helper, beneficiary, t-1) → has_helped_by_time(helper, beneficiary, t)
    /// ```
    ///
    /// Call once per time step through [`ensure_help_tracking`](Self::ensure_help_tracking).
    pub(crate) fn has_helped_by_time_clauses(&mut self, t: usize) -> Vec<Clause> {
        self.ctx.update(t);
        let mut clauses = Vec::new();
        for idx in 0..self.tracked_help_pairs.len() {
            let (helper, beneficiary) = self.tracked_help_pairs[idx];
            let positions = self.help_edge_positions(helper, beneficiary, t);
            let prev = if t > 0 {
                self.pool
                    .get(&VarKey::has_helped_by_time(helper, beneficiary, t - 1))
            } else {
                None
            };
            // Nothing to assert yet: no help event so far and no prefix to carry forward.
            if positions.is_empty() && prev.is_none() {
                continue;
            }
            let has_helped = self.pool.has_helped_by_time(helper, beneficiary, t);
            for pos in &positions {
                let agent_var = self.pool.agent(beneficiary, *pos, t);
                clauses.push(implies(agent_var, has_helped));
            }
            if let Some(prev) = prev {
                clauses.push(implies(prev, has_helped));
            }
        }
        clauses
    }

    /// Clauses and assumptions that forbid *asymmetric* cooperation at horizon `t`.
    ///
    /// A help event `helper → beneficiary` is asymmetric if `helper` is never helped by any other
    /// agent anywhere in the trajectory. This method reifies each concrete asymmetric event into an
    /// [`asymmetric`](VarKey::asymmetric) variable:
    ///
    /// ```text
    /// ¬agent(beneficiary, q, τ) ∨ OR_k has_helped_by_time(k, helper, t) ∨ asymmetric(...)
    /// ```
    ///
    /// The returned assumptions contain `¬asymmetric(...)` for every such variable.
    pub(crate) fn forbid_asymmetric_cooperation(
        &mut self,
        t: usize,
    ) -> (Vec<Clause>, Vec<Literal>) {
        self.ensure_help_tracking(t);
        let mut clauses = Vec::new();
        let mut assumptions = Vec::new();
        for tau in 0..=t {
            self.ctx.update(tau);
            for (helper, beneficiary) in self.tracked_help_pairs.clone() {
                let positions = self.help_edge_positions(helper, beneficiary, tau);
                if positions.is_empty() {
                    continue;
                }
                let incoming: Vec<Literal> = (0..self.ctx.n_agents)
                    .filter(|&other| other != helper)
                    .filter_map(|other| {
                        self.pool.get(&VarKey::has_helped_by_time(other, helper, t))
                    })
                    .collect();
                for pos in positions {
                    let agent_var = self.pool.agent(beneficiary, pos, tau);
                    let asymmetric = self.pool.asymmetric(helper, beneficiary, pos, tau);
                    let mut clause = Vec::with_capacity(incoming.len() + 2);
                    clause.push(-agent_var);
                    clause.extend(incoming.iter().copied());
                    clause.push(asymmetric);
                    clauses.push(clause);
                    assumptions.push(-asymmetric);
                }
            }
        }
        (clauses, assumptions)
    }

    /// Clauses and assumptions that forbid *mutual* cooperation between every pair of agents, at
    /// horizon `t`.
    pub(crate) fn forbid_mutual_cooperation(&mut self, t: usize) -> (Vec<Clause>, Vec<Literal>) {
        self.ensure_help_tracking(t);
        let n_agents = self.ctx.n_agents;
        let mut clauses = Vec::new();
        let mut assumptions = Vec::new();
        for a in 0..n_agents {
            for b in (a + 1)..n_agents {
                let a_helps_b = self.pool.get(&VarKey::has_helped_by_time(a, b, t));
                let b_helps_a = self.pool.get(&VarKey::has_helped_by_time(b, a, t));
                if let (Some(d_ab), Some(d_ba)) = (a_helps_b, b_helps_a) {
                    let mutual = self.pool.mutual(a, b);
                    clauses.push(vec![-d_ab, -d_ba, mutual]);
                    assumptions.push(-mutual);
                }
            }
        }
        (clauses, assumptions)
    }

    /// The progress literal after the first `step` edges of `dependency` have fired by time `t`, or
    /// `None` if that progress variable has not been created yet.
    ///
    /// `step 1` is expressed directly by `has_helped_by_time(dependency[0], dependency[1], t)`;
    /// deeper steps use the chain/cycle-specific progress variable family.
    fn dependency_progress_get(
        &self,
        kind: DependencyKind,
        dependency_id: u32,
        dependency: &[AgentId],
        step: usize,
        t: usize,
    ) -> Option<i32> {
        if step == 1 {
            self.pool
                .get(&VarKey::has_helped_by_time(dependency[0], dependency[1], t))
        } else {
            match kind {
                DependencyKind::Chain { length } => self.pool.get(&VarKey::chain_progress(
                    length,
                    dependency_id,
                    step as u8,
                    t,
                )),
                DependencyKind::Cycle { order } => {
                    self.pool
                        .get(&VarKey::cycle_progress(order, dependency_id, step as u8, t))
                }
            }
        }
    }

    fn dependency_progress_prev_get(
        &self,
        kind: DependencyKind,
        dependency_id: u32,
        step: u8,
        t: usize,
    ) -> Option<i32> {
        if t == 0 {
            return None;
        }
        match kind {
            DependencyKind::Chain { length } => {
                self.pool
                    .get(&VarKey::chain_progress(length, dependency_id, step, t - 1))
            }
            DependencyKind::Cycle { order } => {
                self.pool
                    .get(&VarKey::cycle_progress(order, dependency_id, step, t - 1))
            }
        }
    }

    fn dependency_progress_create(
        &mut self,
        kind: DependencyKind,
        dependency_id: u32,
        step: u8,
        t: usize,
    ) -> i32 {
        match kind {
            DependencyKind::Chain { length } => {
                self.pool.chain_progress(length, dependency_id, step, t)
            }
            DependencyKind::Cycle { order } => {
                self.pool.cycle_progress(order, dependency_id, step, t)
            }
        }
    }

    fn dependency_realized_create(&mut self, kind: DependencyKind, dependency_id: u32) -> i32 {
        match kind {
            DependencyKind::Chain { length } => self.pool.chain_realized(length, dependency_id),
            DependencyKind::Cycle { order } => self.pool.cycle_realized(order, dependency_id),
        }
    }

    fn dependency_clauses(
        &mut self,
        kind: DependencyKind,
        dependencies: &[Vec<AgentId>],
        t: usize,
    ) -> Vec<Clause> {
        self.ctx.update(t);
        let mut clauses = Vec::new();

        for (id, dependency) in dependencies.iter().enumerate() {
            // let dependency = dependencies[dependency_id].clone();
            // let id = dependency_id as u32;
            let id = id as u32;
            let m = dependency.len() - 1; // number of edges

            // Interior edges 2..=m-1 produce progress variables.
            for step in 2..m {
                let (helper, ben) = (dependency[step - 1], dependency[step]);
                let cur_step = step as u8;

                // Monotone in time (independent of any new event at t).
                if let Some(prev_t) = self.dependency_progress_prev_get(kind, id, cur_step, t) {
                    let prog = self.dependency_progress_create(kind, id, cur_step, t);
                    clauses.push(implies(prev_t, prog));
                }

                // progress(step-1, t) ∧ agent(ben, q, t) → progress(step, t).
                let Some(prev_prog) =
                    self.dependency_progress_get(kind, id, dependency, step - 1, t)
                else {
                    continue;
                };
                for pos in self.help_edge_positions(helper, ben, t) {
                    let av = self.pool.agent(ben, pos, t);
                    let prog = self.dependency_progress_create(kind, id, cur_step, t);
                    clauses.push(vec![-prev_prog, -av, prog]);
                }
            }

            // Closing/last edge u_{m-1} → u_m fires the realized indicator.
            let Some(last_prog) = self.dependency_progress_get(kind, id, dependency, m - 1, t)
            else {
                continue;
            };
            let (helper, ben) = (dependency[m - 1], dependency[m]);
            for pos in self.help_edge_positions(helper, ben, t) {
                let av = self.pool.agent(ben, pos, t);
                let realized = self.dependency_realized_create(kind, id);
                clauses.push(vec![-last_prog, -av, realized]);
            }
        }

        clauses
    }

    pub(crate) fn chain_clauses(
        &mut self,
        length: usize,
        chains: &[Vec<AgentId>],
        t: usize,
    ) -> Vec<Clause> {
        self.dependency_clauses(DependencyKind::Chain { length }, chains, t)
    }

    pub(crate) fn cycle_rotation_clauses(
        &mut self,
        order: usize,
        cycle_rotations: &[Vec<AgentId>],
        t: usize,
    ) -> Vec<Clause> {
        self.dependency_clauses(DependencyKind::Cycle { order }, cycle_rotations, t)
    }

    /// Assumptions `¬chain_realized(chain)` for every chain whose realized variable was created.
    pub(crate) fn forbid_chains(&self, length: usize) -> (Vec<Clause>, Vec<Literal>) {
        let Some(support) = self.chain_support.get(&length) else {
            return (vec![], vec![]);
        };
        let assumptions = (0..support.chains.len())
            .filter_map(|id| self.pool.get(&VarKey::chain_realized(length, id as u32)))
            .map(|v| -v)
            .collect();
        (vec![], assumptions)
    }

    /// Assumptions `¬cycle_realized(rotation)` for every cycle rotation whose realized variable was
    /// created.
    pub(crate) fn forbid_cycle_rotations(&self, order: usize) -> (Vec<Clause>, Vec<Literal>) {
        let Some(support) = self.interdependence_support.get(&order) else {
            return (vec![], vec![]);
        };
        let assumptions = (0..support.possible_cycles.len())
            .filter_map(|id| self.pool.get(&VarKey::cycle_realized(order, id as u32)))
            .map(|v| -v)
            .collect();
        (vec![], assumptions)
    }
}

#[cfg(test)]
#[path = "../../unit_tests/test_cooperation_clauses.rs"]
mod tests;
