use crate::{World, solver::VarKey};

use super::ClauseEngine;

/// Build an engine whose potential cooperation graph contains chained help edges.
fn chain_engine(t_max: usize) -> ClauseEngine {
    let world = World::try_from(
        "
         @  S0  S1  S2
        L0E  .   .   .
        L1E  .   .   .
        L2E  .   .   .
         @   X   X   X
        ",
    )
    .expect("failed to parse chain world");
    ClauseEngine::new(&world, t_max)
}

/// Chain clauses use private progress states and materialized help literals only.
#[test]
fn chain_clauses_use_private_progress_states() {
    let t_max = 6;
    let mut engine = chain_engine(t_max);
    for t in 0..=t_max {
        engine.generate_movement_clauses(t);
        engine.generate_help_clauses(t);
    }

    let clauses = (0..=t_max)
        .flat_map(|t| engine.generate_chain_clauses(t, 3))
        .collect::<Vec<_>>();

    assert!(!clauses.is_empty());
    assert!(clauses.iter().flatten().all(|literal| {
        matches!(
            engine.pool.key(literal.unsigned_abs() as i32),
            Some(VarKey::ChainProgress { .. }) | Some(VarKey::Help { .. })
        )
    }));
}

/// Completed chain patterns are blocked directly without allocating final progress states.
///
/// @ai-generated
#[test]
fn chain_clauses_omit_final_progress_states() {
    let t_max = 6;
    let length = 3;
    let mut engine = chain_engine(t_max);
    for t in 0..=t_max {
        engine.generate_movement_clauses(t);
        engine.generate_help_clauses(t);
        engine.generate_chain_clauses(t, length);
    }

    let pattern_count = engine.chain_patterns(length).len();
    for pattern in 0..pattern_count {
        for t in 0..=t_max {
            assert!(!engine.pool.exists(&VarKey::ChainProgress {
                length,
                pattern,
                prefix_len: length,
                t,
            }));
        }
    }
}

/// Missing help literals make their chain transitions impossible instead of causing phantom edges.
///
/// @ai-generated
#[test]
fn chain_clauses_tolerate_missing_help_literals() {
    let mut engine = chain_engine(3);
    for t in 0..=3 {
        engine.generate_movement_clauses(t);
        assert!(engine.generate_chain_clauses(t, 2).is_empty());
    }
}

/// A chain length above the number of available temporal edges emits no blocking clauses.
///
/// @ai-generated
#[test]
fn oversized_chain_length_emits_no_clauses() {
    let t_max = 10;
    let mut engine = chain_engine(t_max);
    for t in 0..=t_max {
        engine.generate_movement_clauses(t);
        engine.generate_help_clauses(t);
        assert!(engine.generate_chain_clauses(t, 100).is_empty());
    }
}
