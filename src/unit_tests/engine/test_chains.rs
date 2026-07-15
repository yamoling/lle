use std::collections::HashSet;

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

/// Chain clauses contain exactly `k` negative help literals and emit no duplicates.
#[test]
fn chain_clauses_are_negative_help_literals_and_deduplicated() {
    let t_max = 6;
    let mut engine = chain_engine(t_max);
    for t in 0..=t_max {
        engine.generate_movement_clauses(t);
        engine.generate_help_clauses(t);
    }

    let clauses = (0..=t_max)
        .flat_map(|t| engine.generate_chain_clauses(t, 2))
        .collect::<Vec<_>>();
    assert!(!clauses.is_empty());
    assert!(clauses.iter().all(|clause| clause.len() == 2));
    assert!(clauses.iter().flatten().all(|literal| {
        *literal < 0 && matches!(engine.pool.key(-*literal), Some(VarKey::Help { .. }))
    }));

    let unique = clauses.iter().cloned().collect::<HashSet<_>>();
    assert_eq!(unique.len(), clauses.len());
}

/// Chain clause generation fails when the required help literals have not been generated.
#[test]
#[should_panic(expected = "Missing help literals!")]
fn chain_clauses_require_help_literals() {
    let mut engine = chain_engine(10);
    for t in 0..=10 {
        engine.generate_movement_clauses(t);
    }
    engine.generate_chain_clauses(5, 2);
}

/// A chain length above the number of available temporal edges emits no blocking clauses.
#[test]
fn oversized_chain_length_emits_no_clauses() {
    let t_max = 10;
    let mut engine = chain_engine(t_max);
    for t in 0..=t_max {
        engine.generate_movement_clauses(t);
        engine.generate_help_clauses(t);
        // Note: there can be trails of length >= t at a given time step t because
        // one agent can shield multiple other agents
        let clauses = engine.generate_chain_clauses(t, 100);
        assert!(clauses.is_empty());
    }
}
