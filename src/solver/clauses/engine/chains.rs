use std::collections::HashSet;

use crate::solver::{Clause, Literal, clauses::ClauseEngine};

impl ClauseEngine {
    /// Generate blocking clauses for temporal help trails of exactly `length` ending at `t`.
    ///
    /// Each candidate trail produces `(¬h₁ ∨ ... ∨ ¬hₖ)`. Help clauses for every step through `t`
    /// must be generated first so all geometrically possible `Help` variables are materialized.
    pub fn generate_chain_clauses(&mut self, t: usize, length: usize) -> Vec<Clause> {
        self.ctx.update(t);
        let trails = self
            .ctx
            .potential_cooperation
            .enumerate_trails_ending_at(length, t);
        // Distinct ordered trails can contain the same temporal edges when simultaneous edges admit
        // multiple valid orderings or cycle rotations. Those trails encode the same unordered SAT
        // clause, so retain only one canonical clause for each temporal-edge set.
        let mut clauses = HashSet::new();

        for trail in trails {
            let mut literals = trail
                .into_iter()
                .map(|edge| self.pool.get(&edge.into()).map(|lit| -lit))
                .collect::<Option<Vec<Literal>>>()
                .expect("Missing help literals!");
            // Canonicalize literal order so the HashSet can collapse equivalent trail orderings.
            literals.sort_unstable();
            clauses.insert(literals);
        }
        clauses.into_iter().collect()
    }
}

#[cfg(test)]
#[path = "../../../unit_tests/engine/test_chains.rs"]
mod tests;
