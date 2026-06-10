use super::generator::ClauseGenerator;
use super::utils::implies;
use super::{Clause, Literal, VarKey};
use crate::{AgentId, Position};

impl ClauseGenerator {
    /// Additive clauses defining the per-pair dependency indicators for time step `t`.
    ///
    /// For every laser owner `helper` and every other agent `beneficiary`, a beneficiary that
    /// legally stands on a relevant tile of the helper's beam can only be safe there because the
    /// helper blocks that beam upstream — that is enforced by [`no_step_on_active_laser`], which
    /// forbids any other-colour agent from standing on an *active* beam tile. Such an occupancy
    /// therefore *is* a help event, and we record it with the implication
    ///
    /// ```text
    /// agent(beneficiary, q, t) → depends_on(beneficiary, helper)
    /// ```
    ///
    /// Only this `←` direction is needed: a real help event forces the indicator true, while the
    /// solver is free to leave it false otherwise. This is exactly what
    /// [`forbid_mutual_cooperation`](Self::forbid_mutual_cooperation) relies on.
    ///
    /// These clauses are *additive* to [`generate`](Self::generate) (they only reference already
    /// existing per-step agent variables) and are only needed when reasoning about who helps
    /// whom. Call once per time step, after `generate(t)`.
    ///
    /// [`no_step_on_active_laser`]: Self::no_step_on_active_laser
    pub fn dependency_clauses(&mut self, t: usize) -> Vec<Clause> {
        self.ctx.update(t);
        // Collect every (beneficiary, helper, position) help event while borrowing `ctx`
        // immutably, then materialise the variables (which borrows `pool` mutably).
        let mut events: Vec<(AgentId, AgentId, Position)> = Vec::new();
        let n_agents = self.ctx.n_agents;
        let sources: Vec<(AgentId, usize)> = self
            .ctx
            .laser_sources
            .iter()
            .map(|s| (s.agent_id, s.laser_id))
            .collect();
        for (helper, laser_id) in sources {
            let beam = self.ctx.relevant_laser_tiles(laser_id, t);
            for beneficiary in 0..n_agents {
                if beneficiary == helper {
                    continue;
                }
                let reachable = self.ctx.relevant_positions_for_agent(beneficiary, t);
                for pos in beam.intersection(reachable) {
                    events.push((beneficiary, helper, pos));
                }
            }
        }
        events
            .into_iter()
            .map(|(beneficiary, helper, pos)| {
                let agent_var = self.pool.agent(beneficiary, pos, t);
                let depends = self.pool.depends_on(beneficiary, helper);
                implies(agent_var, depends)
            })
            .collect()
    }

    /// Clauses and assumptions that forbid *mutual* cooperation between every pair of agents.
    ///
    /// Mutual cooperation between `a` and `b` is the conjunction "`a` helps `b` at some point
    /// **and** `b` helps `a` at some point", i.e. `depends_on(b, a) ∧ depends_on(a, b)`. For
    /// every unordered pair whose mutual dependency is even expressible (both directions have a
    /// defining clause, produced by [`dependency_clauses`](Self::dependency_clauses)), this
    /// reifies the conjunction into a [`mutual`](VarKey::mutual) variable
    ///
    /// ```text
    /// depends_on(b, a) ∧ depends_on(a, b) → mutual(a, b)
    /// ```
    ///
    /// and returns `¬mutual(a, b)` as a solver assumption. Add the returned clauses to the
    /// formula and pass the returned literals as assumptions: any plan exhibiting mutual help is
    /// then ruled out, so an UNSAT result means mutual cooperation is *required* within the
    /// horizon explored so far.
    ///
    /// Call after [`dependency_clauses`](Self::dependency_clauses) has been generated for every
    /// time step of interest (it scans the dependency variables created so far).
    pub fn forbid_mutual_cooperation(&mut self) -> (Vec<Clause>, Vec<Literal>) {
        let n_agents = self.ctx.n_agents;
        let mut clauses = Vec::new();
        let mut assumptions = Vec::new();
        for a in 0..n_agents {
            for b in (a + 1)..n_agents {
                // `a` helps `b` ⇒ depends_on(b, a); `b` helps `a` ⇒ depends_on(a, b).
                let a_helps_b = self.pool.get(&VarKey::depends_on(b, a));
                let b_helps_a = self.pool.get(&VarKey::depends_on(a, b));
                if let (Some(d_ab), Some(d_ba)) = (a_helps_b, b_helps_a) {
                    let mutual = self.pool.mutual(a, b);
                    clauses.push(vec![-d_ab, -d_ba, mutual]);
                    assumptions.push(-mutual);
                }
            }
        }
        (clauses, assumptions)
    }
}

#[cfg(test)]
#[path = "../../unit_tests/test_cooperation_clauses.rs"]
mod tests;
