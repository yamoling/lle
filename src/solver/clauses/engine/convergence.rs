use itertools::Itertools;

use super::utils::implies;
use crate::solver::{Clause, Literal, VarKey, clauses::ClauseEngine};

impl ClauseEngine {
    /// Encode one pairwise-help summary per geometrically possible directed pair through `horizon`.
    ///
    /// Each summary is equivalent to the disjunction of the pair's materialized help literals in
    /// `0..=horizon`. Pairs without a possible help event are not allocated.
    pub fn generate_pairwise_help_clauses(&mut self, horizon: usize) -> Vec<Clause> {
        let mut clauses = Vec::new();
        for beneficiary in 0..self.ctx.n_agents {
            for helper in 0..self.ctx.n_agents {
                if helper == beneficiary {
                    continue;
                }
                let help_variables =
                    self.pool
                        .help_variables_for_pair(helper, beneficiary, horizon);
                if help_variables.is_empty() {
                    continue;
                }
                let pairwise_help = self.pool.pairwise_help(helper, beneficiary, horizon);
                let mut forward = Vec::with_capacity(1 + help_variables.len());
                forward.push(-pairwise_help);
                forward.extend(help_variables.iter().copied());
                clauses.push(forward);
                // Backward clauses
                clauses.extend(
                    help_variables
                        .into_iter()
                        .map(|help| implies(help, pairwise_help)),
                );
            }
        }
        clauses
    }

    /// Forbid any beneficiary from receiving help from at least `k` distinct helpers.
    ///
    /// Pairwise-help equivalences for the same `horizon` must be generated first. Every emitted
    /// clause is one size-`k` combination of negative pairwise-help literals for one beneficiary.
    pub fn generate_no_convergence_clauses(&self, horizon: usize, k: usize) -> Vec<Clause> {
        let mut clauses = Vec::new();
        for beneficiary in 0..self.ctx.n_agents {
            let pairwise_variables: Vec<Literal> = (0..self.ctx.n_agents)
                .filter(|&helper| helper != beneficiary)
                .filter_map(|helper| {
                    self.pool.get(&VarKey::PairwiseHelp {
                        helper,
                        beneficiary,
                        horizon,
                    })
                })
                .collect();
            // Enforce "at most k - 1 helpers" by forbidding every size-k subset from being true
            // simultaneously. Each subset becomes the clause (¬r₁ ∨ ... ∨ ¬rₖ), while
            // `combinations` ensures that helper order does not produce duplicate clauses.
            clauses.extend(
                pairwise_variables
                    .into_iter()
                    .combinations(k)
                    .map(|combination| combination.into_iter().map(|lit| -lit).collect()),
            );
        }
        clauses
    }
}

#[cfg(test)]
#[path = "../../../unit_tests/engine/test_convergence.rs"]
mod tests;
