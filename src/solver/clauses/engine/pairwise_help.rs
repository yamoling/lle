use itertools::Itertools;

use super::utils::implies;
use crate::solver::{Clause, Literal, VarKey, clauses::ClauseEngine};

/// Which endpoint of the directed `helper -> beneficiary` relation is held fixed while grouping
/// pairwise-help summaries into degree-blocking clauses.
///
/// The relation is directed, so `r(i, j, H)` and `r(j, i, H)` are distinct variables. Convergence
/// fixes the beneficiary and varies the helpers; divergence fixes the helper and varies the
/// beneficiaries. Naming the orientation avoids silently swapping the two endpoints.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum FixedEndpoint {
    /// Group the outgoing summaries of one helper (k-divergence).
    Helper,
    /// Group the incoming summaries of one beneficiary (k-convergence).
    Beneficiary,
}

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

    /// Require at least one ordered pair of distinct agents to lack help through `horizon`.
    ///
    /// This is the direct negative encoding of fully coupled cooperation. If every ordered pair has
    /// a summary, one clause `¬r(0,1) ∨ ¬r(0,2) ∨ ...` requires a missing relationship. If there are
    /// fewer than two agents, or any pair has no summary because help is geometrically impossible,
    /// the profile is already false and no restriction is needed.
    ///
    /// Pairwise-help equivalences for the same `horizon` must be generated first.
    pub fn generate_no_fully_coupled_clauses(&self, horizon: usize) -> Vec<Clause> {
        if self.ctx.n_agents < 2 {
            return Vec::new();
        }

        let mut missing_pair = Vec::with_capacity(self.ctx.n_agents * (self.ctx.n_agents - 1));
        for helper in 0..self.ctx.n_agents {
            for beneficiary in 0..self.ctx.n_agents {
                if helper == beneficiary {
                    continue;
                }
                let Some(summary) = self.pool.get(&VarKey::PairwiseHelp {
                    helper,
                    beneficiary,
                    horizon,
                }) else {
                    return Vec::new();
                };
                missing_pair.push(-summary);
            }
        }
        vec![missing_pair]
    }

    /// Forbid any agent from being incident to `k` distinct pairwise-help summaries at `horizon`.
    ///
    /// `fixed` selects the endpoint held constant: the emitted clauses then enforce "at most
    /// `k - 1` distinct agents on the varying endpoint". Every clause is one size-`k` combination
    /// of negative pairwise-help literals sharing the fixed endpoint, so the resulting count is
    /// `Σ_anchor C(b_anchor, k)` where `b_anchor` is the number of allocated summaries incident to
    /// that agent in the chosen direction.
    ///
    /// Pairwise-help equivalences for the same `horizon` must be generated first. Nothing is
    /// emitted when `k` is structurally unreachable (`k >= n_agents`) or below the meaningful
    /// threshold (`k < 2`); the latter case is additionally rejected by `ClauseGenerator`.
    pub(super) fn generate_degree_blocking_clauses(
        &self,
        horizon: usize,
        k: usize,
        fixed: FixedEndpoint,
    ) -> Vec<Clause> {
        // An agent has at most `n_agents - 1` distinct counterparts, so a threshold at or above
        // the agent count can never be reached and no combination has to be enumerated.
        if k < 2 || k >= self.ctx.n_agents {
            return Vec::new();
        }
        let mut clauses = Vec::new();
        for anchor in 0..self.ctx.n_agents {
            let summaries: Vec<Literal> = (0..self.ctx.n_agents)
                .filter(|&other| other != anchor)
                .filter_map(|other| {
                    let (helper, beneficiary) = match fixed {
                        FixedEndpoint::Helper => (anchor, other),
                        FixedEndpoint::Beneficiary => (other, anchor),
                    };
                    self.pool.get(&VarKey::PairwiseHelp {
                        helper,
                        beneficiary,
                        horizon,
                    })
                })
                .collect();
            // Enforce "at most k - 1 counterparts" by forbidding every size-k subset from being
            // true simultaneously. Each subset becomes the clause (¬r₁ ∨ ... ∨ ¬rₖ), while
            // `combinations` ensures that agent order does not produce duplicate clauses.
            clauses.extend(
                summaries
                    .into_iter()
                    .combinations(k)
                    .map(|combination| combination.into_iter().map(|lit| -lit).collect()),
            );
        }
        clauses
    }
}

#[cfg(test)]
#[path = "../../../unit_tests/engine/test_pairwise_help.rs"]
mod tests;
