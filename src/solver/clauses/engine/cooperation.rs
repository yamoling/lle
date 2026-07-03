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
    /// the beneficiary can actually reach at least one such beam tile at `t` (i.e. the movement
    /// layer already created its agent variable); otherwise the help event is geometrically
    /// impossible and no variable or clause is emitted.
    pub fn help_clauses(&mut self, t: usize) -> Vec<Clause> {
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
                for &pos in &source.path {
                    if let Some(benef_in_laser_pos) = self.pool.get(&VarKey::Agent {
                        agent_id: beneficiary,
                        pos,
                        t,
                    }) {
                        benef_positions_in_laser.push(benef_in_laser_pos);
                    }
                }
            }
            // No reachable beam tile: the help event is impossible, so create nothing (neither a
            // `Help` variable nor any clause).
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

    /// Encode whether any help event is asymmetric within the prefix `0..=horizon`.
    ///
    /// For each concrete help event `help(i, j, t)`, the clause
    /// `¬help(i, j, t) ∨ asymmetric ∨ incoming_help_to_i` says that if agent `i` helps someone
    /// while no one helps `i` anywhere in the same horizon, the global `Asymmetric` variable must be
    /// true. `NoAsymmetricCooperation` then forbids that variable by assumption.
    ///
    /// This method must run after the help buffer has created all `Help` variables up to `horizon`.
    /// It only probes existing help variables, so it never creates unconstrained future help events.
    pub fn encode_asymmetry(&mut self, horizon: usize) -> Vec<Clause> {
        self.ctx.update(horizon);
        let asymmetric = self.pool.asymmetric();
        let mut clauses = Vec::new();
        for t in 0..=horizon {
            // Iterate only over geometrically possible help events at this time step. The potential
            // graph tells us which directed help edges can exist; the variable pool tells us which
            // ones were actually materialized by `help_clauses`.
            for edge in self.ctx.potential_cooperation.edges_at(t) {
                let Some(help) = self.pool.get(&crate::solver::VarKey::Help {
                    helper: edge.helper,
                    beneficiary: edge.beneficiary,
                    t,
                }) else {
                    continue;
                };

                // Gather every concrete help event, anywhere in the horizon, where `edge.helper`
                // is the beneficiary. If this list is empty, then `edge.helper` was never helped.
                let mut incoming_help = Vec::new();
                for t2 in 0..=horizon {
                    for incoming_helper in self
                        .ctx
                        .potential_cooperation
                        .at(t2)
                        .incoming_to(edge.helper)
                    {
                        if let Some(incoming) = self.pool.get(&crate::solver::VarKey::Help {
                            helper: incoming_helper,
                            beneficiary: edge.helper,
                            t: t2,
                        }) {
                            incoming_help.push(incoming);
                        }
                    }
                }

                // Encode: help(edge.helper -> edge.beneficiary at t) ∧ no incoming help to
                // edge.helper over the whole horizon -> asymmetric.
                //
                // CNF form:
                //   ¬help ∨ asymmetric ∨ incoming_help_1 ∨ ... ∨ incoming_help_n
                //
                // If `help` is true and every incoming help literal is false, the only way to satisfy
                // this clause is to set the global `asymmetric` variable to true. The
                // `NoAsymmetricCooperation` mode then forbids such plans by assuming `¬asymmetric`.
                let mut clause = Vec::with_capacity(incoming_help.len() + 2);
                clause.push(-help);
                clause.push(asymmetric);
                clause.extend(incoming_help);
                clauses.push(clause);
            }
        }
        clauses
    }

    /// Return the assumption that forbids asymmetric help events.
    pub fn assume_no_asymmetry(&mut self, _horizon: usize) -> Vec<Literal> {
        vec![-self.pool.asymmetric()]
    }
}

#[cfg(test)]
#[path = "../../../unit_tests/engine/test_help.rs"]
mod tests;
