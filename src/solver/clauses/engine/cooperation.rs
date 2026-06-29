use std::collections::HashMap;

use super::utils::implies;
use crate::{
    AgentId, Position,
    solver::{Clause, clauses::ClauseEngine, position_set::PositionSet},
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
}

#[cfg(test)]
#[path = "../../../unit_tests/engine/test_help.rs"]
mod tests;
