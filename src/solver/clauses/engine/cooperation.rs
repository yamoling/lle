use std::collections::HashMap;

use super::utils::implies;
use crate::{
    AgentId, Position,
    solver::{Clause, Literal, clauses::ClauseEngine, position_set::PositionSet},
};

impl ClauseEngine {
    /// Return each beneficiary beam position and the upstream helper positions that can make it safe.
    fn help_support(
        &self,
        helper: AgentId,
        beneficiary: AgentId,
        t: usize,
    ) -> HashMap<Position, PositionSet> {
        let helper_reachable = self.ctx.relevant_positions_for_agent(helper, t);
        let beneficiary_reachable = self.ctx.relevant_positions_for_agent(beneficiary, t);
        let mut support = HashMap::new();
        let height = self.ctx.height();
        let width = self.ctx.width();
        for source in self
            .ctx
            .laser_sources
            .iter()
            .filter(|source| source.agent_id == helper)
        {
            let relevant_laser_tiles = self.ctx.relevant_laser_tiles(source.laser_id, t);
            let mut upstream_blockers = PositionSet::empty(height, width);
            for pos in &source.path {
                if beneficiary_reachable.contains(pos) && relevant_laser_tiles.contains(pos) {
                    support
                        .entry(*pos)
                        .or_insert_with(|| PositionSet::empty(height, width))
                        .union_with(&upstream_blockers);
                }
                if helper_reachable.contains(pos) {
                    upstream_blockers.insert(*pos);
                }
            }
        }
        support
    }

    /// Encode `Help(helper, beneficiary, t)` as the event where the beneficiary stands on one of the
    /// helper's beam tiles and the helper occupies an upstream beam tile that can block it.
    pub fn help_clauses(&mut self, t: usize) -> Vec<Clause> {
        let mut clauses = Vec::new();
        for edge in self.ctx.potential_cooperation.edges_at(t) {
            let support = self.help_support(edge.helper, edge.beneficiary, t);
            // Only create help variables and clauses when relevant
            if support.is_empty() {
                continue;
            }
            let help = self.pool.help(edge.helper, edge.beneficiary, t);
            let mut beneficiary_literals = Vec::with_capacity(support.len());
            for (beneficiary_pos, helper_positions) in support {
                // If the beneficiary stands on one of the helper's relevant beam tiles, then a
                // concrete help event occurred: beneficiary_at_pos -> help.
                let beneficiary_at_pos = self.pool.agent(edge.beneficiary, beneficiary_pos, t);
                beneficiary_literals.push(beneficiary_at_pos);
                clauses.push(implies(beneficiary_at_pos, help));

                // For this beneficiary beam tile, collect every upstream position where the helper
                // could block the beam. Multiple laser sources can contribute the same blocker, so
                // keep the clause compact by sorting and deduplicating the literals.
                let helper_blockers: Vec<i32> = helper_positions
                    .iter()
                    .map(|pos| self.pool.agent(edge.helper, pos, t))
                    .collect();

                // If this help event is true because the beneficiary is on this beam tile, at least
                // one upstream helper blocker must also be occupied:
                // help ∧ beneficiary_at_pos -> OR(helper_blockers).
                let mut helper_blocks_beneficiary = Vec::with_capacity(helper_blockers.len() + 2);
                helper_blocks_beneficiary.push(-help);
                helper_blocks_beneficiary.push(-beneficiary_at_pos);
                helper_blocks_beneficiary.extend(helper_blockers);
                clauses.push(helper_blocks_beneficiary);
            }

            let mut help_requires_beneficiary_on_beam =
                Vec::with_capacity(beneficiary_literals.len() + 1);
            help_requires_beneficiary_on_beam.push(-help);
            help_requires_beneficiary_on_beam.extend(beneficiary_literals);
            clauses.push(help_requires_beneficiary_on_beam);
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
