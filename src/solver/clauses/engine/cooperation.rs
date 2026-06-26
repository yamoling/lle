use super::utils::implies;
use crate::{
    AgentId, Position,
    solver::{Clause, Literal, VarKey, clauses::ClauseEngine},
};

impl ClauseEngine {
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

    /// Clauses that reify asymmetric cooperation at exactly time step `t`.
    ///
    /// A help event `helper → beneficiary` is asymmetric when `helper` has not been helped by any
    /// other agent by the same time step. Each concrete event is encoded into an
    /// [`asymmetric`](VarKey::asymmetric) variable:
    ///
    /// ```text
    /// ¬agent(beneficiary, q, t) ∨ OR_k has_helped_by_time(k, helper, t) ∨ asymmetric(...)
    /// ```
    ///
    /// The solve mode forbids these events separately by assuming every generated asymmetric
    /// variable to be false.
    pub fn make_asymmetric_clauses(&mut self, t: usize) -> Vec<Clause> {
        self.ctx.update(t);
        let mut clauses = Vec::new();
        for (helper, beneficiary) in self.tracked_help_pairs.clone() {
            let positions = self.help_edge_positions(helper, beneficiary, t);
            if positions.is_empty() {
                continue;
            }
            let incoming: Vec<Literal> = (0..self.ctx.n_agents)
                .filter(|&other| other != helper)
                .filter_map(|other| self.pool.get(&VarKey::has_helped_by_time(other, helper, t)))
                .collect();
            for pos in positions {
                let agent_var = self.pool.agent(beneficiary, pos, t);
                let asymmetric = self.pool.asymmetric(helper, beneficiary, pos, t);
                let mut clause = Vec::with_capacity(incoming.len() + 2);
                clause.push(-agent_var);
                clause.extend(incoming.iter().copied());
                clause.push(asymmetric);
                clauses.push(clause);
            }
        }
        clauses
    }

    /// Assumptions forbidding asymmetric cooperation variables generated for exactly time step `t`.
    pub fn assume_no_asymmetric_at(&mut self, t: usize) -> Vec<Literal> {
        self.ctx.update(t);
        let mut assumptions = Vec::new();
        for (helper, beneficiary) in self.tracked_help_pairs.clone() {
            for pos in self.help_edge_positions(helper, beneficiary, t) {
                assumptions.push(-self.pool.asymmetric(helper, beneficiary, pos, t));
            }
        }
        assumptions
    }
}
