use crate::{World, solver::VarKey};

use super::{ClauseEngine, FixedEndpoint};

/// Build a three-agent engine for testing pairwise clauses independently of world geometry.
fn pairwise_engine(t_max: usize) -> ClauseEngine {
    let world = World::try_from("S0 S1 S2 X X X").expect("failed to parse pairwise world");
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

/// Return whether an assignment of the help literals and the summary satisfies every clause.
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

/// The summary clauses are satisfied exactly when the summary equals the prefix disjunction.
///
/// @ai-generated
#[test]
fn pairwise_summary_truth_table_matches_the_help_disjunction() {
    let mut engine = pairwise_engine(5);
    let helps = [
        engine.pool.help(0, 2, 0),
        engine.pool.help(0, 2, 1),
        engine.pool.help(0, 2, 3),
    ];
    let clauses = engine.generate_pairwise_help_clauses(3);
    let summary = engine
        .literal(&summary_key(0, 2, 3))
        .expect("pairwise-help must exist");

    for mask in 0..(1u8 << helps.len()) {
        for summary_value in [false, true] {
            let mut assignment: Vec<i32> = helps
                .iter()
                .enumerate()
                .filter(|(index, _)| mask & (1 << index) != 0)
                .map(|(_, &help)| help)
                .collect();
            if summary_value {
                assignment.push(summary);
            }
            let expected = (mask != 0) == summary_value;
            assert_eq!(
                satisfies(&clauses, &assignment),
                expected,
                "mask {mask}, summary {summary_value}"
            );
        }
    }
}

/// Pairwise-help summaries encode both directions of the prefix disjunction and ignore later help.
#[test]
fn pairwise_help_is_equivalent_to_materialized_help_prefix() {
    let mut engine = pairwise_engine(5);
    let help_at_one = engine.pool.help(0, 2, 1);
    let help_at_three = engine.pool.help(0, 2, 3);
    let help_after_horizon = engine.pool.help(0, 2, 5);

    let clauses = engine.generate_pairwise_help_clauses(3);
    let key = summary_key(0, 2, 3);
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
    assert!(!engine.exists(&summary_key(1, 2, 3)));
}

/// Help at `t = 0` belongs to every horizon and help beyond the horizon belongs to none.
///
/// @ai-generated
#[test]
fn summaries_only_include_their_own_prefix() {
    let mut engine = pairwise_engine(5);
    let help_at_zero = engine.pool.help(0, 1, 0);
    let help_at_four = engine.pool.help(0, 1, 4);

    let short = engine.generate_pairwise_help_clauses(2);
    let short_summary = engine
        .literal(&summary_key(0, 1, 2))
        .expect("short summary must exist");
    assert!(short.contains(&vec![-short_summary, help_at_zero]));
    assert!(!short.iter().flatten().any(|&lit| lit == help_at_four));

    let long = engine.generate_pairwise_help_clauses(4);
    let long_summary = engine
        .literal(&summary_key(0, 1, 4))
        .expect("long summary must exist");
    assert!(long.contains(&vec![-long_summary, help_at_zero, help_at_four]));
}

/// Summary variables are distinguished by direction and by horizon.
///
/// @ai-generated
#[test]
fn summary_keys_are_directed_and_horizon_scoped() {
    let mut engine = pairwise_engine(5);
    engine.pool.help(0, 1, 1);
    engine.pool.help(1, 0, 2);

    engine.generate_pairwise_help_clauses(3);
    engine.generate_pairwise_help_clauses(4);

    let forward = engine.literal(&summary_key(0, 1, 3)).expect("0 -> 1 at 3");
    let backward = engine.literal(&summary_key(1, 0, 3)).expect("1 -> 0 at 3");
    let later = engine.literal(&summary_key(0, 1, 4)).expect("0 -> 1 at 4");

    assert_ne!(forward, backward);
    assert_ne!(forward, later);
}

/// Regenerating one horizon reuses the same variables and produces the same clauses.
///
/// @ai-generated
#[test]
fn repeated_generation_at_one_horizon_is_idempotent() {
    let mut engine = pairwise_engine(4);
    engine.pool.help(0, 1, 1);
    engine.pool.help(2, 1, 2);

    let first = engine.generate_pairwise_help_clauses(3);
    let variables_after_first = engine.n_vars();
    let second = engine.generate_pairwise_help_clauses(3);

    assert_eq!(first, second);
    assert_eq!(engine.n_vars(), variables_after_first);
}

/// Both orientations of the shared grouping select the intended endpoint.
///
/// @ai-generated
#[test]
fn fixed_endpoint_selects_the_grouping_orientation() {
    let mut engine = pairwise_engine(4);
    engine.pool.help(0, 1, 1);
    engine.pool.help(0, 2, 2);
    engine.generate_pairwise_help_clauses(3);

    let outgoing = engine.generate_degree_blocking_clauses(3, 2, FixedEndpoint::Helper);
    let incoming = engine.generate_degree_blocking_clauses(3, 2, FixedEndpoint::Beneficiary);

    let to_one = engine.literal(&summary_key(0, 1, 3)).expect("0 -> 1");
    let to_two = engine.literal(&summary_key(0, 2, 3)).expect("0 -> 2");
    assert_eq!(outgoing, vec![vec![-to_one, -to_two]]);
    assert!(incoming.is_empty());
}
