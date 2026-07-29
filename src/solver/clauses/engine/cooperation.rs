use itertools::Itertools;

use super::utils::implies;
use crate::solver::{Clause, Literal, VarKey, clauses::ClauseEngine};

impl ClauseEngine {
    /// Generate `Help(helper, beneficiary, t)` variables for time step `t`.
    ///
    /// A help event `helper → beneficiary` at `t` means the beneficiary stands on one of the
    /// helper's laser beam tiles — an occupancy that is only survivable when the helper blocks that
    /// beam upstream (enforced by the laser clauses, not here). The formula is
    ///
    /// ```text
    /// agent(beneficiary, pos, t) → help(helper, beneficiary, t)      for every reachable beam tile
    /// help(helper, beneficiary, t) → OR_pos agent(beneficiary, pos, t)
    /// ```
    ///
    /// Beam positions are pooled across **all** of the helper's laser sources, so a single `Help`
    /// variable covers every beam the helper owns. A `Help` variable is materialized **only** when
    /// the beneficiary can reach a beam tile that the helper can actually make safe by blocking
    /// upstream at `t`; otherwise the help event is geometrically impossible and no variable or
    /// clause is emitted.
    pub fn generate_help_clauses(&mut self, t: usize) -> Vec<Clause> {
        self.ctx.update(t);
        let mut clauses = vec![];
        for agents in (0..self.ctx.n_agents).permutations(2) {
            let helper = agents[0];
            let beneficiary = agents[1];
            // Beneficiary occupancies that lie on one of the helper's beams and are actually
            // reachable, i.e. the movement layer already materialized their agent variable.
            let mut benef_positions_in_laser = vec![];
            for source in self
                .ctx
                .laser_sources
                .iter()
                .filter(|source| source.agent_id == helper)
            {
                for pos in self.ctx.relevant_laser_tiles(source.laser_id, t) {
                    if let Some(benef_in_laser_pos) = self.pool.get(&VarKey::Agent {
                        agent_id: beneficiary,
                        pos,
                        t,
                    }) {
                        benef_positions_in_laser.push(benef_in_laser_pos);
                    }
                }
            }
            // No reachable beam tile: the help event is impossible, so create nothing
            if benef_positions_in_laser.is_empty() {
                continue;
            }
            // Crossing beams can share a tile; keep every literal at most once.
            benef_positions_in_laser.sort_unstable();
            benef_positions_in_laser.dedup();
            let help = self.pool.help(helper, beneficiary, t);
            // agent_in_position -> help
            for &benef_in_laser_pos in &benef_positions_in_laser {
                clauses.push(implies(benef_in_laser_pos, help));
            }
            // help -> OR(agent in one position)
            let mut clause = Vec::with_capacity(1 + benef_positions_in_laser.len());
            clause.push(-help);
            clause.extend(benef_positions_in_laser);
            clauses.push(clause);
        }
        clauses
    }

    /// Encode `is_helped(beneficiary) <=> OR(help_variables)` over the prefix `0..=horizon`.
    ///
    /// This function must be called for each horizon.
    pub fn generate_is_helped(&mut self, horizon: usize) -> Vec<Clause> {
        let mut clauses = vec![];
        for beneficiary in 0..self.ctx.n_agents {
            // Only create the "is_helped" variable if the agent can actually be helped.
            let beneficiary_variables = self.pool.beneficiary_variables(beneficiary, horizon);
            if beneficiary_variables.is_empty() {
                continue;
            }
            let is_helped = self.pool.is_helped(beneficiary, horizon);
            // is_helped <=> OR_{beneficiary variables}
            // 1) Forward: is_helped -> OR_{beneficiary variables}
            let mut forward = Vec::with_capacity(1 + beneficiary_variables.len());
            forward.push(-is_helped);
            forward.extend(beneficiary_variables.iter());
            clauses.push(forward);
            // 2) Backward: b -> is_helped (for all b in beneficiary variables)
            for v in beneficiary_variables {
                clauses.push(implies(v, is_helped));
            }
        }
        clauses
    }

    /// Encode `provides_help(helper) <=> OR(helper_variables)` over the prefix `0..=horizon`.
    ///
    /// This function should be called for each time horizon.
    pub fn generate_provides_help(&mut self, horizon: usize) -> Vec<Clause> {
        let mut clauses = vec![];
        for helper in 0..self.ctx.n_agents {
            let helper_variables = self.pool.helper_variables(helper, horizon);
            if helper_variables.is_empty() {
                continue;
            }
            let provides_help = self.pool.provides_help_up_to(helper, horizon);
            // provides_help <=> OR(helper_variables)
            // 1) Forward: provides_help => OR(helper_variables)
            let mut forward = Vec::with_capacity(1 + helper_variables.len());
            forward.push(-provides_help);
            forward.extend(helper_variables.iter());
            clauses.push(forward);
            // 2) Backward: h -> provides_help (for all h in helper variables)
            for v in helper_variables {
                clauses.push(implies(v, provides_help));
            }
        }
        clauses
    }

    /// Encode the global `asymmetric` flag over the prefix `0..=horizon`.
    ///
    /// Asymmetry is defined as: there exists an agent that provides help but is never helped, i.e.
    /// ```text
    /// asymmetric <=> OR_{i in A} (provides_help(i) ∧ ¬is_helped(i))
    /// ```
    pub fn encode_asymmetry(&mut self, horizon: usize) -> Vec<Clause> {
        let asymmetric = self.pool.asymmetric(horizon);
        let mut clauses = Vec::new();
        // Collects one literal per agent-level disjunct of the outer OR.
        let mut disjuncts = Vec::with_capacity(self.ctx.n_agents);
        for i in 0..self.ctx.n_agents {
            // Skip agents that can never provide help
            if !self
                .pool
                .exists(&VarKey::ProvidesHelp { helper: i, horizon })
            {
                continue;
            }
            let provides_help = self
                .pool
                .get(&VarKey::ProvidesHelp { helper: i, horizon })
                .expect("Help variables not initialized");
            let beneficiary_variables = self.pool.beneficiary_variables(i, horizon);
            // Agents that can never be helped have no `is_helped` variable,
            // therefore `¬is_helped` is always true and the conjunction reduces to `provides_help`.
            if beneficiary_variables.is_empty() {
                clauses.push(implies(provides_help, asymmetric));
                disjuncts.push(provides_help);
                continue;
            }
            // A conjunction cannot appear bare inside a CNF clause, so each disjunct
            // `provides_help(i) ∧ ¬is_helped(i)` is reified into an auxiliary variable `a_i`.
            let is_helped = self
                .pool
                .get(&VarKey::IsHelped {
                    beneficiary: i,
                    horizon,
                })
                .expect("is_helped variable not initialized");
            // Reify a_i <=> provides_help(i) ∧ ¬is_helped(i).
            let a = self.pool.aux();
            clauses.push(implies(a, provides_help)); // a -> provides_help
            clauses.push(implies(a, -is_helped)); // a -> ¬is_helped
            clauses.push(vec![-provides_help, is_helped, a]); // provides_help ∧ ¬is_helped -> a
            // Backward: a -> asymmetric.
            clauses.push(implies(a, asymmetric));
            disjuncts.push(a);
        }
        // Forward: asymmetric -> OR a_i. With no disjuncts this is the unit clause `¬asymmetric`,
        // pinning the flag to false since no agent can ever be asymmetrically helped.
        let mut forward = Vec::with_capacity(1 + disjuncts.len());
        forward.push(-asymmetric);
        forward.extend(disjuncts);
        clauses.push(forward);
        clauses
    }

    /// Return the assumption that forbids asymmetric help events.
    pub fn assume_no_asymmetry(&mut self, horizon: usize) -> Vec<Literal> {
        vec![-self.pool.asymmetric(horizon)]
    }
}

#[cfg(test)]
#[path = "../../../unit_tests/engine/test_help.rs"]
mod tests;
