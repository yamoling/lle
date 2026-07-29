use crate::{World, solver::VarKey};

use super::ClauseEngine;

/// Build a three-agent engine for testing divergence clauses independently of world geometry.
fn divergence_engine(t_max: usize) -> ClauseEngine {
    let world = World::try_from("S0 S1 S2 X X X").expect("failed to parse divergence world");
    ClauseEngine::new(&world, t_max)
}

/// Build a four-agent engine for testing blocker counts.
fn four_agent_engine(t_max: usize) -> ClauseEngine {
    let world = World::try_from("S0 S1 S2 S3 X X X X").expect("failed to parse divergence world");
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

/// Return whether an assignment satisfies every clause of a formula.
///
/// @ai-generated
fn satisfies(clauses: &[Vec<i32>], true_literals: &[i32]) -> bool {
    clauses.iter().all(|clause| {
        clause.iter().any(|&literal| {
            if literal > 0 {
                true_literals.contains(&literal)
            } else {
                !true_literals.contains(&-literal)
            }
        })
    })
}

/// Every blocker holds one helper fixed and negates `k` distinct beneficiaries.
///
/// @ai-generated
#[test]
fn blockers_share_a_helper_and_have_distinct_beneficiaries() {
    let mut engine = four_agent_engine(4);
    engine.pool.help(0, 1, 1);
    engine.pool.help(0, 2, 2);
    engine.pool.help(0, 3, 3);
    engine.generate_pairwise_help_clauses(3);

    let clauses = engine.generate_no_divergence_clauses(3, 2);
    for clause in &clauses {
        assert_eq!(clause.len(), 2);
        let mut beneficiaries = Vec::new();
        for &literal in clause {
            assert!(literal < 0, "blocker literals must be negative");
            match engine
                .pool
                .key(-literal)
                .expect("literal must decode to a key")
            {
                VarKey::PairwiseHelp {
                    helper,
                    beneficiary,
                    horizon,
                } => {
                    assert_eq!(helper, 0);
                    assert_eq!(horizon, 3);
                    beneficiaries.push(beneficiary);
                }
                other => panic!("unexpected key {other:?}"),
            }
        }
        beneficiaries.sort_unstable();
        beneficiaries.dedup();
        assert_eq!(beneficiaries.len(), 2);
    }
}

/// Exhaustive assignments show that the formula forbids exactly `k` true outgoing summaries.
///
/// @ai-generated
#[test]
fn divergence_blockers_encode_at_most_k_minus_one_beneficiaries() {
    let mut engine = four_agent_engine(4);
    engine.pool.help(0, 1, 1);
    engine.pool.help(0, 2, 2);
    engine.pool.help(0, 3, 3);
    engine.generate_pairwise_help_clauses(3);

    let summaries = [
        engine.literal(&summary_key(0, 1, 3)).expect("0 -> 1"),
        engine.literal(&summary_key(0, 2, 3)).expect("0 -> 2"),
        engine.literal(&summary_key(0, 3, 3)).expect("0 -> 3"),
    ];

    for k in [2usize, 3] {
        let clauses = engine.generate_no_divergence_clauses(3, k);
        for mask in 0..(1u8 << summaries.len()) {
            let assignment: Vec<i32> = summaries
                .iter()
                .enumerate()
                .filter(|(index, _)| mask & (1 << index) != 0)
                .map(|(_, &lit)| lit)
                .collect();
            assert_eq!(
                satisfies(&clauses, &assignment),
                assignment.len() < k,
                "k = {k}, mask = {mask}"
            );
        }
    }
}

/// Repeated help to one beneficiary yields a single summary and no blocker.
///
/// @ai-generated
#[test]
fn repeated_help_to_one_beneficiary_emits_no_blocker() {
    let mut engine = divergence_engine(5);
    engine.pool.help(0, 1, 1);
    engine.pool.help(0, 1, 3);
    engine.generate_pairwise_help_clauses(4);

    assert!(engine.exists(&summary_key(0, 1, 4)));
    assert!(!engine.exists(&summary_key(0, 2, 4)));
    assert!(engine.generate_no_divergence_clauses(4, 2).is_empty());
}

/// Simultaneous help to two beneficiaries emits the two-literal blocker.
///
/// @ai-generated
#[test]
fn simultaneous_help_to_two_beneficiaries_emits_one_blocker() {
    let mut engine = divergence_engine(4);
    engine.pool.help(0, 1, 2);
    engine.pool.help(0, 2, 2);
    engine.generate_pairwise_help_clauses(3);

    let to_one = engine.literal(&summary_key(0, 1, 3)).expect("0 -> 1");
    let to_two = engine.literal(&summary_key(0, 2, 3)).expect("0 -> 2");
    assert_eq!(
        engine.generate_no_divergence_clauses(3, 2),
        vec![vec![-to_one, -to_two]]
    );
}

/// Help at `t = 0` and help at different times produce the same outgoing grouping.
///
/// @ai-generated
#[test]
fn help_at_different_times_produces_the_same_grouping() {
    let mut engine = divergence_engine(5);
    engine.pool.help(0, 1, 0);
    engine.pool.help(0, 2, 4);
    engine.generate_pairwise_help_clauses(4);

    let to_one = engine.literal(&summary_key(0, 1, 4)).expect("0 -> 1");
    let to_two = engine.literal(&summary_key(0, 2, 4)).expect("0 -> 2");
    assert_eq!(
        engine.generate_no_divergence_clauses(4, 2),
        vec![vec![-to_one, -to_two]]
    );
}

/// Two helpers of one beneficiary converge without diverging.
///
/// @ai-generated
#[test]
fn convergent_help_emits_no_divergence_blocker() {
    let mut engine = divergence_engine(4);
    engine.pool.help(0, 2, 1);
    engine.pool.help(1, 2, 3);
    engine.generate_pairwise_help_clauses(3);

    assert!(engine.generate_no_divergence_clauses(3, 2).is_empty());
    assert_eq!(engine.generate_no_convergence_clauses(3, 2).len(), 1);
}

/// A helper with three beneficiaries emits `C(3, k)` blockers, and none at the agent count.
///
/// @ai-generated
#[test]
fn four_agent_blocker_counts_follow_the_combination_formula() {
    let mut engine = four_agent_engine(4);
    engine.pool.help(0, 1, 1);
    engine.pool.help(0, 2, 2);
    engine.pool.help(0, 3, 3);
    engine.generate_pairwise_help_clauses(3);

    assert_eq!(engine.generate_no_divergence_clauses(3, 2).len(), 3);
    assert_eq!(engine.generate_no_divergence_clauses(3, 3).len(), 1);
    assert!(engine.generate_no_divergence_clauses(3, 4).is_empty());
}

/// A threshold at the agent count is structurally unreachable and emits nothing.
///
/// @ai-generated
#[test]
fn threshold_at_the_agent_count_emits_no_blocker() {
    let mut engine = divergence_engine(4);
    engine.pool.help(0, 1, 1);
    engine.pool.help(0, 2, 1);
    engine.generate_pairwise_help_clauses(3);

    assert!(engine.generate_no_divergence_clauses(3, 3).is_empty());
    assert!(engine.generate_no_convergence_clauses(3, 3).is_empty());
}
