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
    pub(crate) fn dependency_clauses(&mut self, t: usize) -> Vec<Clause> {
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
    pub(crate) fn forbid_mutual_cooperation(&mut self) -> (Vec<Clause>, Vec<Literal>) {
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

    /// Additive clauses for chain detection at time step `t`.
    ///
    /// Builds two families of indicator variables:
    ///
    /// 1. **`first_helped_by_time(helper, beneficiary, t)`** — "helper has helped beneficiary at
    ///    any time step ≤ t". Seeded by the same beam-occupancy implications as
    ///    [`dependency_clauses`](Self::dependency_clauses), then propagated forward in time:
    ///    ```text
    ///    agent(beneficiary, q, t) → first_helped_by_time(helper, beneficiary, t)
    ///    first_helped_by_time(helper, beneficiary, t-1) → first_helped_by_time(helper, beneficiary, t)
    ///    ```
    ///
    /// 2. **`chain_event(a, b, c, t)`** and **`chain(a, b, c)`** — for every triple `(a, b, c)`
    ///    where `b ≠ a` and `b ≠ c` (c may equal a for cycles): if `a` has helped `b` by time `t`
    ///    and `b` helps `c` at time `t`, a chain event is recorded and lifted to the horizon-level
    ///    chain indicator:
    ///    ```text
    ///    first_helped_by_time(a, b, t) ∧ agent(c, q, t) → chain_event(a, b, c, t)
    ///    chain_event(a, b, c, t) → chain(a, b, c)
    ///    ```
    ///
    /// Call once per time step, after [`dependency_clauses`](Self::dependency_clauses), before
    /// [`forbid_chained_cooperation`](Self::forbid_chained_cooperation).
    pub(crate) fn chain_clauses(&mut self, t: usize) -> Vec<Clause> {
        self.ctx.update(t);
        let mut clauses = Vec::new();
        let n_agents = self.ctx.n_agents;

        let sources: Vec<(AgentId, usize)> = self
            .ctx
            .laser_sources
            .iter()
            .map(|s| (s.agent_id, s.laser_id))
            .collect();

        // Part 1: first_helped_by_time
        for &(helper, laser_id) in &sources {
            let beam = self.ctx.relevant_laser_tiles(laser_id, t);
            for beneficiary in 0..n_agents {
                if beneficiary == helper {
                    continue;
                }
                let reachable = self.ctx.relevant_positions_for_agent(beneficiary, t);
                let positions: Vec<Position> = beam.intersection(reachable).collect();
                if positions.is_empty() {
                    continue;
                }

                let fhbt = self.pool.first_helped_by_time(helper, beneficiary, t);

                // agent(beneficiary, q, t) → first_helped_by_time(helper, beneficiary, t)
                for pos in &positions {
                    let agent_var = self.pool.agent(beneficiary, *pos, t);
                    clauses.push(vec![-agent_var, fhbt]);
                }

                // first_helped_by_time(helper, beneficiary, t-1) → first_helped_by_time(helper, beneficiary, t)
                if t > 0 {
                    if let Some(prev) =
                        self.pool.get(&VarKey::first_helped_by_time(helper, beneficiary, t - 1))
                    {
                        clauses.push(vec![-prev, fhbt]);
                    }
                }
            }
        }

        // Part 2: chain_event and chain
        // For each triple (a, b, c): b has a laser, b ≠ a, b ≠ c (a may equal c for cycles).
        for &(b, b_laser_id) in &sources {
            let b_beam = self.ctx.relevant_laser_tiles(b_laser_id, t);

            for a in 0..n_agents {
                if a == b {
                    continue;
                }
                let Some(fhbt_ab) = self.pool.get(&VarKey::first_helped_by_time(a, b, t)) else {
                    continue;
                };

                for c in 0..n_agents {
                    if c == b {
                        continue;
                    }
                    let c_reachable = self.ctx.relevant_positions_for_agent(c, t);
                    let c_positions: Vec<Position> = b_beam.intersection(c_reachable).collect();
                    if c_positions.is_empty() {
                        continue;
                    }

                    let chain_ev = self.pool.chain_event(a, b, c, t);
                    let chain_var = self.pool.chain(a, b, c);

                    // first_helped_by_time(a, b, t) ∧ agent(c, q, t) → chain_event(a, b, c, t)
                    for pos in &c_positions {
                        let agent_c = self.pool.agent(c, *pos, t);
                        clauses.push(vec![-fhbt_ab, -agent_c, chain_ev]);
                    }

                    // chain_event(a, b, c, t) → chain(a, b, c)
                    clauses.push(vec![-chain_ev, chain_var]);
                }
            }
        }

        clauses
    }

    /// Assumptions that forbid *any* chained cooperation across all agent triples.
    ///
    /// A chain `a → b → c` is represented by the [`VarKey::Chain`] indicator created by
    /// [`chain_clauses`](Self::chain_clauses). This method scans all such variables and returns
    /// `¬chain(a, b, c)` as a solver assumption for every triple whose chain variable exists.
    /// Add the returned assumptions alongside those from `generate(t)`: any plan that contains
    /// a temporal chain is then ruled out, so an UNSAT result means chained cooperation is
    /// required within the horizon.
    ///
    /// Call after [`chain_clauses`](Self::chain_clauses) has been generated for every time step.
    pub(crate) fn forbid_chained_cooperation(&self) -> (Vec<Clause>, Vec<Literal>) {
        let n_agents = self.ctx.n_agents;
        let mut assumptions = Vec::new();
        for a in 0..n_agents {
            for b in 0..n_agents {
                if b == a {
                    continue;
                }
                for c in 0..n_agents {
                    if c == b {
                        continue;
                    }
                    if let Some(chain_var) = self.pool.get(&VarKey::chain(a, b, c)) {
                        assumptions.push(-chain_var);
                    }
                }
            }
        }
        (vec![], assumptions)
    }
}

#[cfg(test)]
#[path = "../../unit_tests/test_cooperation_clauses.rs"]
mod tests;
