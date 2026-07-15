use std::collections::HashSet;

use crate::solver::{Clause, Literal, clauses::ClauseEngine};

impl ClauseEngine {
    /// Generate blocking clauses for temporal cycles of exactly `order` whose closing edge is
    /// at `t`.
    ///
    /// Each candidate cycle produces `(¬h₁ ∨ ... ∨ ¬hₖ)`. Help clauses for every step through `t`
    /// must be generated first so all geometrically possible `Help` variables are materialized.
    pub fn generate_cycle_clauses(&mut self, t: usize, order: usize) -> Vec<Clause> {
        self.ctx.update(t);
        let cycles = self
            .ctx
            .potential_cooperation
            .enumerate_cycles_ending_at(order, t);
        // Rotations of a simultaneous cycle are distinct edge sequences over the same temporal
        // edges. Those cycles encode the same unordered SAT clause, so retain only one canonical
        // clause for each temporal-edge set.
        let mut clauses = HashSet::new();

        for cycle in cycles {
            let mut literals = cycle
                .into_iter()
                .map(|edge| self.pool.get(&edge.into()).map(|lit| -lit))
                .collect::<Option<Vec<Literal>>>()
                .expect("Missing help literals!");
            // Canonicalize literal order so the HashSet can collapse equivalent cycle rotations.
            literals.sort_unstable();
            clauses.insert(literals);
        }
        clauses.into_iter().collect()
    }
}

#[cfg(test)]
#[path = "../../../unit_tests/engine/test_cycles.rs"]
mod tests;
