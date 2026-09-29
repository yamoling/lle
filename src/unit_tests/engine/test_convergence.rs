use crate::{World, solver::VarKey};

use super::ClauseEngine;

/// Build a three-agent engine for testing convergence clauses independently of world geometry.
fn convergence_engine(t_max: usize) -> ClauseEngine {
    let world = World::try_from("S0 S1 S2 X X X").expect("failed to parse convergence world");
    ClauseEngine::new(&world, t_max)
}

/// Return the summary key of one directed pair at one horizon.
fn summary_key(helper: usize, beneficiary: usize, horizon: usize) -> VarKey {
    VarKey::PairwiseHelp {
        helper,
        beneficiary,
        horizon,
    }
}

/// A beneficiary with fewer than `k` possible helpers emits no convergence blocker.
#[test]
fn insufficient_distinct_helpers_emit_no_blocking_clause() {
    let mut engine = convergence_engine(3);
    engine.pool.help(0, 2, 1);
    engine.pool.help(0, 2, 2);

    engine.generate_pairwise_help_clauses(3);
    assert!(engine.generate_no_convergence_clauses(3, 2).is_empty());
    assert!(engine.generate_no_convergence_clauses(3, 4).is_empty());
}

/// Two helpers of one beneficiary emit exactly one blocker over their incoming summaries.
///
/// @ai-generated
#[test]
fn two_distinct_helpers_emit_one_incoming_blocker() {
    let mut engine = convergence_engine(4);
    engine.pool.help(0, 2, 1);
    engine.pool.help(1, 2, 3);
    engine.generate_pairwise_help_clauses(3);

    let from_zero = engine.literal(&summary_key(0, 2, 3)).expect("0 -> 2");
    let from_one = engine.literal(&summary_key(1, 2, 3)).expect("1 -> 2");
    assert_eq!(
        engine.generate_no_convergence_clauses(3, 2),
        vec![vec![-from_zero, -from_one]]
    );
}

/// Convergence blockers group incoming summaries, so pure divergence emits none.
///
/// @ai-generated
#[test]
fn outgoing_help_emits_no_convergence_blocker() {
    let mut engine = convergence_engine(4);
    engine.pool.help(0, 1, 1);
    engine.pool.help(0, 2, 2);
    engine.generate_pairwise_help_clauses(3);

    assert!(engine.generate_no_convergence_clauses(3, 2).is_empty());
    assert_eq!(engine.generate_no_divergence_clauses(3, 2).len(), 1);
}
