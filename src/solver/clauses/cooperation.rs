use super::generator::ClauseGenerator;
use super::utils::implies;
use super::{Clause, Literal, VarKey};
use crate::{AgentId, Position};

impl ClauseGenerator {
    /// Positions where `beneficiary` can legally stand on one of `helper`'s beams at time `t`.
    ///
    /// Such an occupancy is a *help edge* `helper → beneficiary`: [`no_step_on_active_laser`]
    /// forbids any non-owner from standing on an *active* beam tile, so the only way `beneficiary`
    /// can be there is if `helper` blocks the beam upstream. This single primitive replaces the
    /// `beam ∩ reachable` intersection that every cooperation-aware mode used to open-code.
    ///
    /// [`no_step_on_active_laser`]: Self::no_step_on_active_laser
    fn help_edge_positions(&self, helper: AgentId, beneficiary: AgentId, t: usize) -> Vec<Position> {
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

    /// Clauses defining `first_helped_by_time(helper, beneficiary, t)` for every pair that can
    /// help, at time step `t`. This is a **monotone temporal prefix-OR**: it becomes true at the
    /// first help event and stays true forever after.
    ///
    /// ```text
    /// agent(beneficiary, q, t)                    → first_helped_by_time(helper, beneficiary, t)
    /// first_helped_by_time(helper, beneficiary, t-1) → first_helped_by_time(helper, beneficiary, t)
    /// ```
    ///
    /// This single family serves three purposes, replacing what used to be three separate
    /// variable towers:
    /// - read at the current horizon it is the time-agnostic "h ever helps b" indicator that the
    ///   mutual-cooperation forbid relies on (formerly `DependsOn`);
    /// - it is the first edge (`step 1`) of every temporal walk (formerly `CycleProgress(·,1,·)`);
    /// - it is the left side of every chain (its original role).
    ///
    /// Call once per time step, after `generate(t)`.
    pub(crate) fn first_helped_by_time_clauses(&mut self, t: usize) -> Vec<Clause> {
        self.ctx.update(t);
        let mut clauses = Vec::new();
        for idx in 0..self.fhbt_pairs.len() {
            let (helper, beneficiary) = self.fhbt_pairs[idx];
            let positions = self.help_edge_positions(helper, beneficiary, t);
            let prev = if t > 0 {
                self.pool
                    .get(&VarKey::first_helped_by_time(helper, beneficiary, t - 1))
            } else {
                None
            };
            // Nothing to assert yet: no help event so far and no prefix to carry forward.
            if positions.is_empty() && prev.is_none() {
                continue;
            }
            let fhbt = self.pool.first_helped_by_time(helper, beneficiary, t);
            for pos in &positions {
                let agent_var = self.pool.agent(beneficiary, *pos, t);
                clauses.push(implies(agent_var, fhbt));
            }
            if let Some(prev) = prev {
                clauses.push(implies(prev, fhbt));
            }
        }
        clauses
    }

    /// Clauses and assumptions that forbid *mutual* cooperation between every pair of agents, at
    /// horizon `t`.
    ///
    /// Mutual cooperation between `a` and `b` is "`a` helps `b` at some point **and** `b` helps
    /// `a` at some point". With the monotone [`first_helped_by_time`](Self::first_helped_by_time_clauses)
    /// indicator read at the current horizon, that is exactly
    /// `first_helped_by_time(a, b, t) ∧ first_helped_by_time(b, a, t)`, reified into a
    /// [`mutual`](VarKey::mutual) variable
    ///
    /// ```text
    /// first_helped_by_time(a, b, t) ∧ first_helped_by_time(b, a, t) → mutual(a, b)
    /// ```
    ///
    /// and returned together with the assumption `¬mutual(a, b)`. Call after
    /// `first_helped_by_time_clauses` has been generated for every time step `0..=t`.
    pub(crate) fn forbid_mutual_cooperation(&mut self, t: usize) -> (Vec<Clause>, Vec<Literal>) {
        let n_agents = self.ctx.n_agents;
        let mut clauses = Vec::new();
        let mut assumptions = Vec::new();
        for a in 0..n_agents {
            for b in (a + 1)..n_agents {
                // `a` helps `b` ⇒ first_helped_by_time(a, b); `b` helps `a` ⇒ first_helped_by_time(b, a).
                let a_helps_b = self.pool.get(&VarKey::first_helped_by_time(a, b, t));
                let b_helps_a = self.pool.get(&VarKey::first_helped_by_time(b, a, t));
                if let (Some(d_ab), Some(d_ba)) = (a_helps_b, b_helps_a) {
                    let mutual = self.pool.mutual(a, b);
                    clauses.push(vec![-d_ab, -d_ba, mutual]);
                    assumptions.push(-mutual);
                }
            }
        }
        (clauses, assumptions)
    }

    /// The progress literal after the first `step` edges of `walk` have fired by time `t`, or
    /// `None` if that progress variable has not been created yet.
    ///
    /// `step 1` is expressed directly by [`first_helped_by_time`](Self::first_helped_by_time_clauses)
    /// (the walk's first edge `walk[0] → walk[1]`); deeper steps use the dedicated
    /// [`WalkProgress`](VarKey::WalkProgress) family.
    fn walk_progress_get(&self, walk_id: u32, walk: &[AgentId], step: usize, t: usize) -> Option<i32> {
        if step == 1 {
            self.pool
                .get(&VarKey::first_helped_by_time(walk[0], walk[1], t))
        } else {
            self.pool.get(&VarKey::walk_progress(walk_id, step as u8, t))
        }
    }

    /// Additive clauses, at time step `t`, advancing every temporal walk in `self.walks`.
    ///
    /// A walk is a vertex sequence `[u0, u1, …, um]` (edge `i` is `u_i → u_{i+1}`); chains are open
    /// walks `[a, b, c]` and interdependence cycles are closed walks `[v0, …, v_{m-1}, v0]`. The
    /// encoding is one construction for both:
    ///
    /// ```text
    /// // step 1 is first_helped_by_time(u0, u1, ·) — emitted by first_helped_by_time_clauses
    /// progress(s-1, t) ∧ agent(u_s, q, t) → progress(s, t)      for 2 ≤ s ≤ m-1   (interior edges)
    /// progress(s, t-1)                     → progress(s, t)                        (monotone in t)
    /// progress(m-1, t) ∧ agent(u_m, q, t) → walk_realized(walk) (the closing/last edge)
    /// ```
    ///
    /// where `agent(u_s, q, t)` ranges over the help-edge tiles of `u_{s-1}`'s beam reachable by
    /// `u_s`. Call once per time step, after
    /// [`first_helped_by_time_clauses`](Self::first_helped_by_time_clauses).
    pub(crate) fn walk_clauses(&mut self, t: usize) -> Vec<Clause> {
        self.ctx.update(t);
        let mut clauses = Vec::new();

        for walk_id in 0..self.walks.len() {
            let walk = self.walks[walk_id].clone();
            let id = walk_id as u32;
            let m = walk.len() - 1; // number of edges

            // Interior edges 2..=m-1 produce WalkProgress variables.
            for step in 2..m {
                let (helper, ben) = (walk[step - 1], walk[step]);
                let cur_step = step as u8;

                // Monotone in time (independent of any new event at t).
                let prev_t = if t > 0 {
                    self.pool.get(&VarKey::walk_progress(id, cur_step, t - 1))
                } else {
                    None
                };
                if let Some(prev_t) = prev_t {
                    let prog = self.pool.walk_progress(id, cur_step, t);
                    clauses.push(implies(prev_t, prog));
                }

                // progress(step-1, t) ∧ agent(ben, q, t) → progress(step, t).
                let Some(prev_prog) = self.walk_progress_get(id, &walk, step - 1, t) else {
                    continue;
                };
                for pos in self.help_edge_positions(helper, ben, t) {
                    let av = self.pool.agent(ben, pos, t);
                    let prog = self.pool.walk_progress(id, cur_step, t);
                    clauses.push(vec![-prev_prog, -av, prog]);
                }
            }

            // Closing (last) edge u_{m-1} → u_m fires walk_realized.
            let Some(last_prog) = self.walk_progress_get(id, &walk, m - 1, t) else {
                continue;
            };
            let (helper, ben) = (walk[m - 1], walk[m]);
            for pos in self.help_edge_positions(helper, ben, t) {
                let av = self.pool.agent(ben, pos, t);
                let realized = self.pool.walk_realized(id);
                clauses.push(vec![-last_prog, -av, realized]);
            }
        }

        clauses
    }

    /// Assumptions `¬walk_realized(walk)` for every walk whose realized variable was created by
    /// [`walk_clauses`](Self::walk_clauses). Forbidding all of them rules out any temporal chain
    /// (chained mode) or any temporal cycle (interdependence mode). Pass the returned
    /// assumptions alongside those from `generate(t)`.
    pub(crate) fn forbid_walks(&self) -> (Vec<Clause>, Vec<Literal>) {
        let assumptions = (0..self.walks.len())
            .filter_map(|id| self.pool.get(&VarKey::walk_realized(id as u32)))
            .map(|v| -v)
            .collect();
        (vec![], assumptions)
    }
}

#[cfg(test)]
#[path = "../../unit_tests/test_cooperation_clauses.rs"]
mod tests;
