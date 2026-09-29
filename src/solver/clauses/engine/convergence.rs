use super::pairwise_help::FixedEndpoint;
use crate::solver::{Clause, clauses::ClauseEngine};

impl ClauseEngine {
    /// Forbid any beneficiary from receiving help from at least `k` distinct helpers.
    ///
    /// Pairwise-help equivalences for the same `horizon` must be generated first. Every emitted
    /// clause is one size-`k` combination of negative pairwise-help literals for one beneficiary.
    pub fn generate_no_convergence_clauses(&self, horizon: usize, k: usize) -> Vec<Clause> {
        self.generate_degree_blocking_clauses(horizon, k, FixedEndpoint::Beneficiary)
    }
}

#[cfg(test)]
#[path = "../../../unit_tests/engine/test_convergence.rs"]
mod tests;
