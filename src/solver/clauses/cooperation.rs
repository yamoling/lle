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

    /// Clauses defining `has_helped_by_time(helper, beneficiary, t)` for every pair that can
    /// help, at time step `t`. This is a **monotone temporal prefix-OR**: it becomes true at the
    /// first help event and stays true forever after.
    ///
    /// ```text
    /// agent(beneficiary, q, t)                    → has_helped_by_time(helper, beneficiary, t)
    /// has_helped_by_time(helper, beneficiary, t-1) → has_helped_by_time(helper, beneficiary, t)
    /// ```
    ///
    /// This single family serves multiple purposes:
    /// - read at the current horizon it is the time-agnostic "h ever helps b" indicator that the
    ///   mutual-cooperation forbid relies on (formerly `DependsOn`);
    /// - it is the first edge (`step 1`) of every temporal trail used by interdependence and
    ///   chain modes (formerly `CycleProgress(·,1,·)`).
    ///
    /// Call once per time step, after `generate(t)`.
    pub(crate) fn has_helped_by_time_clauses(&mut self, t: usize) -> Vec<Clause> {
        self.ctx.update(t);
        let mut clauses = Vec::new();
        for idx in 0..self.has_helped_pairs.len() {
            let (helper, beneficiary) = self.has_helped_pairs[idx];
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
    /// agent(beneficiary, q, τ) ∧ no incoming help into helper by t → asymmetric(helper, beneficiary, q, τ)
    /// ```
    ///
    /// encoded as:
    ///
    /// ```text
    /// ¬agent(beneficiary, q, τ) ∨ OR_k has_helped_by_time(k, helper, t) ∨ asymmetric(...)
    /// ```
    ///
    /// The returned assumptions contain `¬asymmetric(...)` for every such variable. Under those
    /// assumptions, the clauses reduce to the direct no-asymmetric constraint:
    ///
    /// ```text
    /// agent(beneficiary, q, τ) → OR_k has_helped_by_time(k, helper, t)
    /// ```
    ///
    /// If no incoming-help indicator exists for `helper`, the defining clause is
    /// `¬agent(beneficiary, q, τ) ∨ asymmetric(...)`, and the negative assumption forbids all
    /// outgoing help from that unhelpable helper.
    ///
    /// This must stay horizon-specific and must not be folded into the per-step cached clauses
    /// generated alongside laser activity. The condition that makes an outgoing help event
    /// non-asymmetric is "the helper is helped at some time `≤ t`", where `t` is the current solve
    /// horizon, not the event time `τ`. A helper may help at `τ` and only be helped later. Using
    /// `has_helped_by_time(_, helper, τ)` would reject such valid non-asymmetric trajectories;
    /// using `t_max` would be unsound for shorter horizon solves. Therefore these clauses are
    /// generated at the end of `generate(t)`, after the current horizon is known.
    pub(crate) fn forbid_asymmetric_cooperation(
        &mut self,
        t: usize,
    ) -> (Vec<Clause>, Vec<Literal>) {
        let mut clauses = Vec::new();
        let mut assumptions = Vec::new();
        for tau in 0..=t {
            self.ctx.update(tau);
            for (helper, beneficiary) in self.has_helped_pairs.clone() {
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
    ///
    /// Mutual cooperation between `a` and `b` is "`a` helps `b` at some point **and** `b` helps
    /// `a` at some point". With the monotone [`has_helped_by_time`](Self::has_helped_by_time_clauses)
    /// indicator read at the current horizon, that is exactly
    /// `has_helped_by_time(a, b, t) ∧ has_helped_by_time(b, a, t)`, reified into a
    /// [`mutual`](VarKey::mutual) variable
    ///
    /// ```text
    /// has_helped_by_time(a, b, t) ∧ has_helped_by_time(b, a, t) → mutual(a, b)
    /// ```
    ///
    /// and returned together with the assumption `¬mutual(a, b)`. Call after
    /// `has_helped_by_time_clauses` has been generated for every time step `0..=t`.
    pub(crate) fn forbid_mutual_cooperation(&mut self, t: usize) -> (Vec<Clause>, Vec<Literal>) {
        let n_agents = self.ctx.n_agents;
        let mut clauses = Vec::new();
        let mut assumptions = Vec::new();
        for a in 0..n_agents {
            for b in (a + 1)..n_agents {
                // `a` helps `b` ⇒ has_helped_by_time(a, b); `b` helps `a` ⇒ has_helped_by_time(b, a).
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

    /// The progress literal after the first `step` edges of `trail` have fired by time `t`, or
    /// `None` if that progress variable has not been created yet.
    ///
    /// `step 1` is expressed directly by [`has_helped_by_time`](Self::has_helped_by_time_clauses)
    /// (the trail's first edge `trail[0] → trail[1]`); deeper steps use the dedicated
    /// [`TrailProgress`](VarKey::TrailProgress) family.
    fn trail_progress_get(
        &self,
        trail_id: u32,
        trail: &[AgentId],
        step: usize,
        t: usize,
    ) -> Option<i32> {
        if step == 1 {
            self.pool
                .get(&VarKey::has_helped_by_time(trail[0], trail[1], t))
        } else {
            self.pool
                .get(&VarKey::trail_progress(trail_id, step as u8, t))
        }
    }

    /// Additive clauses, at time step `t`, advancing every temporal interdependence cycle in
    /// `self.trails`.
    ///
    /// A cycle is a closed vertex sequence `[u0, u1, …, um]` (edge `i` is `u_i → u_{i+1}` and
    /// `u_m == u0`):
    ///
    /// ```text
    /// // step 1 is has_helped_by_time(u0, u1, ·) — emitted by has_helped_by_time_clauses
    /// progress(s-1, t) ∧ agent(u_s, q, t) → progress(s, t)      for 2 ≤ s ≤ m-1   (interior edges)
    /// progress(s, t-1)                     → progress(s, t)                        (monotone in t)
    /// progress(m-1, t) ∧ agent(u_m, q, t) → trail_realized(trail) (the closing edge)
    /// ```
    ///
    /// where `agent(u_s, q, t)` ranges over the help-edge tiles of `u_{s-1}`'s beam reachable by
    /// `u_s`. Call once per time step, after
    /// [`has_helped_by_time_clauses`](Self::has_helped_by_time_clauses).
    pub(crate) fn trail_clauses(&mut self, t: usize) -> Vec<Clause> {
        self.ctx.update(t);
        let mut clauses = Vec::new();

        for trail_id in 0..self.trails.len() {
            let trail = self.trails[trail_id].clone();
            let id = trail_id as u32;
            let m = trail.len() - 1; // number of edges

            // Interior edges 2..=m-1 produce TrailProgress variables.
            for step in 2..m {
                let (helper, ben) = (trail[step - 1], trail[step]);
                let cur_step = step as u8;

                // Monotone in time (independent of any new event at t).
                let prev_t = if t > 0 {
                    self.pool.get(&VarKey::trail_progress(id, cur_step, t - 1))
                } else {
                    None
                };
                if let Some(prev_t) = prev_t {
                    let prog = self.pool.trail_progress(id, cur_step, t);
                    clauses.push(implies(prev_t, prog));
                }

                // progress(step-1, t) ∧ agent(ben, q, t) → progress(step, t).
                let Some(prev_prog) = self.trail_progress_get(id, &trail, step - 1, t) else {
                    continue;
                };
                for pos in self.help_edge_positions(helper, ben, t) {
                    let av = self.pool.agent(ben, pos, t);
                    let prog = self.pool.trail_progress(id, cur_step, t);
                    clauses.push(vec![-prev_prog, -av, prog]);
                }
            }

            // Closing (last) edge u_{m-1} → u_m fires trail_realized.
            let Some(last_prog) = self.trail_progress_get(id, &trail, m - 1, t) else {
                continue;
            };
            let (helper, ben) = (trail[m - 1], trail[m]);
            for pos in self.help_edge_positions(helper, ben, t) {
                let av = self.pool.agent(ben, pos, t);
                let realized = self.pool.trail_realized(id);
                clauses.push(vec![-last_prog, -av, realized]);
            }
        }

        clauses
    }

    /// Assumptions `¬trail_realized(trail)` for every interdependence cycle whose realized variable
    /// was created by [`trail_clauses`](Self::trail_clauses). Pass the returned assumptions alongside
    /// those from `generate(t)`.
    pub(crate) fn forbid_trails(&self) -> (Vec<Clause>, Vec<Literal>) {
        let assumptions = (0..self.trails.len())
            .filter_map(|id| self.pool.get(&VarKey::trail_realized(id as u32)))
            .map(|v| -v)
            .collect();
        (vec![], assumptions)
    }
}

#[cfg(test)]
#[path = "../../unit_tests/test_cooperation_clauses.rs"]
mod tests;
