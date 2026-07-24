//! Tests for `VarPool::decode_plan`, the SAT-model -> joint-plan decoder
//! (`solver/clauses/var_pool.rs`).
//!
//! `decode_plan` is tested directly against `VarPool` (no SAT solver needed): agent variables are
//! allocated, a model is synthesized from them, and the decoded actions are checked.

use super::VarPool;
use crate::solver::{VarKey, errors::SolverError};
use crate::{Action, Position};

fn pos(i: usize, j: usize) -> Position {
    Position { i, j }
}

/// Interdependence progress variables are keyed and reused by their semantic identity.
///
/// @ai-generated
#[test]
fn interdependence_progress_uses_a_dedicated_var_key() {
    let mut pool = VarPool::new();
    let first = pool.interdependence_progress(3, 4, 2, 5);
    assert_eq!(first, pool.interdependence_progress(3, 4, 2, 5));
    assert_ne!(first, pool.interdependence_progress(3, 4, 3, 5));
    assert_eq!(
        pool.key(first),
        Some(VarKey::InterdependenceProgress {
            order: 3,
            pattern: 4,
            prefix_len: 2,
            t: 5,
        })
    );
}

/// Pairwise-help variables are uniquely keyed by directed pair and horizon.
#[test]
fn pairwise_help_uses_a_horizon_scoped_var_key() {
    let mut pool = VarPool::new();
    let first = pool.pairwise_help(0, 2, 4);

    assert_eq!(first, pool.pairwise_help(0, 2, 4));
    assert_ne!(first, pool.pairwise_help(1, 2, 4));
    assert_ne!(first, pool.pairwise_help(0, 1, 4));
    assert_ne!(first, pool.pairwise_help(0, 2, 5));
    assert_eq!(
        pool.key(first),
        Some(VarKey::PairwiseHelp {
            helper: 0,
            beneficiary: 2,
            horizon: 4,
        })
    );
}

/// Pair-specific help lookup filters by both agents and horizon and returns deterministic literals.
#[test]
fn help_variables_for_pair_are_filtered_and_sorted() {
    let mut pool = VarPool::new();
    let after_horizon = pool.help(0, 2, 4);
    let at_two = pool.help(0, 2, 2);
    pool.help(1, 2, 1);
    let at_one = pool.help(0, 2, 1);
    pool.help(0, 1, 1);

    let variables = pool.help_variables_for_pair(0, 2, 2);
    let mut expected = vec![at_one, at_two];
    expected.sort_unstable();

    assert_eq!(variables, expected);
    assert_eq!(pool.help_variables_for_pair(0, 2, 2), expected);
    assert!(!variables.contains(&after_horizon));
}

/// `decode_plan` must translate the per-step agent positions of a SAT model back into the joint
/// action sequence, covering every one of the five action directions.
#[test]
fn decode_plan_recovers_all_action_directions() {
    let mut pool = VarPool::new();
    // A single agent walking: Stay, East, West, North, South.
    let path = [
        pos(1, 1),
        pos(1, 1),
        pos(1, 2),
        pos(1, 1),
        pos(0, 1),
        pos(1, 1),
    ];
    let literals: Vec<i32> = path
        .iter()
        .enumerate()
        .map(|(t, &p)| pool.agent(0, p, t))
        .collect();
    let plan = pool
        .decode_plan(&literals, path.len() - 1)
        .expect("a contiguous trajectory must decode");
    assert_eq!(
        plan,
        vec![
            vec![Action::Stay],
            vec![Action::East],
            vec![Action::West],
            vec![Action::North],
            vec![Action::South],
        ]
    );
}

/// A non-adjacent jump between two consecutive steps must be rejected with
/// `SolverError::InvalidTrajectory` rather than silently producing a bogus action.
#[test]
fn decode_plan_rejects_non_adjacent_jump() {
    let mut pool = VarPool::new();
    let literals = vec![pool.agent(0, pos(0, 0), 0), pool.agent(0, pos(0, 2), 1)];
    let result = pool.decode_plan(&literals, 1);
    assert!(
        matches!(
            result,
            Err(SolverError::InvalidTrajectory {
                agent: 0,
                index: 1,
                ..
            })
        ),
        "a two-tile jump must be reported as an invalid trajectory, got {result:?}"
    );
}

/// A model that is missing an agent's position at some intermediate step must be reported with
/// `SolverError::MissingPosition` rather than producing a partial or bogus plan.
#[test]
fn decode_plan_errors_on_missing_timestep() {
    let mut pool = VarPool::new();
    // Provide t=0 and t=2 but omit t=1: a malformed/partial model.
    let literals = vec![pool.agent(0, pos(0, 0), 0), pool.agent(0, pos(0, 0), 2)];
    let result = pool.decode_plan(&literals, 2);
    assert!(
        matches!(result, Err(SolverError::MissingPosition { agent: 0, t: 1 })),
        "a model missing an intermediate step must report MissingPosition, got {result:?}"
    );
}
