use super::pairwise_help::FixedEndpoint;
use crate::solver::{Clause, clauses::ClauseEngine};

impl ClauseEngine {
    /// Forbid any helper from helping at least `k` distinct beneficiaries.
    ///
    /// This is the outgoing dual of [`ClauseEngine::generate_no_convergence_clauses`]. Pairwise-help
    /// equivalences for the same `horizon` must be generated first. Every emitted clause is one
    /// size-`k` combination of negative pairwise-help literals sharing a helper.
    pub fn generate_no_divergence_clauses(&self, horizon: usize, k: usize) -> Vec<Clause> {
        self.generate_degree_blocking_clauses(horizon, k, FixedEndpoint::Helper)
    }
}

#[cfg(test)]
#[path = "../../../unit_tests/engine/test_divergence.rs"]
mod tests;
