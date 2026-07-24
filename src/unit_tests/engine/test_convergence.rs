use crate::{World, solver::VarKey};

use super::ClauseEngine;

/// Build a three-agent engine for testing convergence clauses independently of world geometry.
fn convergence_engine(t_max: usize) -> ClauseEngine {
    let world = World::try_from("S0 S1 S2 X X X").expect("failed to parse convergence world");
    ClauseEngine::new(&world, t_max)
}

/// Pairwise-help summaries encode both directions of the prefix disjunction and ignore later help.
#[test]
fn pairwise_help_is_equivalent_to_materialized_help_prefix() {
    let mut engine = convergence_engine(5);
    let help_at_one = engine.pool.help(0, 2, 1);
    let help_at_three = engine.pool.help(0, 2, 3);
    let help_after_horizon = engine.pool.help(0, 2, 5);

    let clauses = engine.generate_pairwise_help_clauses(3);
    let key = VarKey::PairwiseHelp {
        helper: 0,
        beneficiary: 2,
        horizon: 3,
    };
    let pairwise = engine.literal(&key).expect("pairwise-help must exist");

    let forward = clauses
        .iter()
        .filter(|clause| clause.contains(&-pairwise))
        .collect::<Vec<_>>();
    assert_eq!(forward.len(), 1);
    assert_eq!(forward[0], &vec![-pairwise, help_at_one, help_at_three]);
    assert!(clauses.contains(&vec![-help_at_one, pairwise]));
    assert!(clauses.contains(&vec![-help_at_three, pairwise]));
    assert!(
        !clauses
            .iter()
            .flatten()
            .any(|&lit| lit == help_after_horizon)
    );

    engine.generate_pairwise_help_clauses(3);
    assert_eq!(engine.literal(&key), Some(pairwise));
    assert!(!engine.exists(&VarKey::PairwiseHelp {
        helper: 1,
        beneficiary: 2,
        horizon: 3,
    }));
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
