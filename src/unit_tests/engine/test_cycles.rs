use crate::{
    World,
    solver::{VarKey, interdependence::StaticHelpArc},
};

use super::ClauseEngine;

/// Build an engine whose world geometry admits help arcs.
fn cycle_engine(t_max: usize) -> ClauseEngine {
    let world = World::try_from(
        "
         @  S0  S1  S2
        L0E  .   .   .
        L1E  .   .   .
        L2E  .   .   .
         @   X   X   X
        ",
    )
    .expect("failed to parse cycle world");
    ClauseEngine::new(&world, t_max)
}

/// The canonical eight-agent ring exposes only its eight physically feasible help arcs.
#[test]
fn eight_agent_ring_patterns_exclude_unreachable_help_arcs() {
    let world = World::try_from(
        "
  .   .  .   .   .  .   .   . L1S  .
 L0E S0  X   .   .  .   .   . S1   .
  .   .  .  L5S  .  .   .   .  X   .
  .   .  .  S5   .  X  S4  L4W .   .
  .   .  .   X   .  .   .   .  .   .
  .   X  .   .   .  .   X   .  .   .
  X  S7  X  S6  L6W .  S3   X S2  L2W
  .  L7N .   .   .  .  L3N  .  .   .
",
    )
    .unwrap();
    let mut engine = ClauseEngine::new(&world, 1);
    let expected = (0..8)
        .map(|helper| StaticHelpArc {
            helper,
            beneficiary: (helper + 1) % 8,
        })
        .collect::<Vec<_>>();

    assert_eq!(engine.potential_help_arcs(), expected);
    let patterns = engine.interdependence_patterns(8);
    assert_eq!(patterns.len(), 8);
    assert!(patterns.iter().all(|pattern| pattern.arcs.len() == 8));
}

/// Interdependence clauses use only private auxiliaries and materialized help literals.
#[test]
fn interdependence_clauses_use_private_progress_states() {
    let t_max = 6;
    let mut engine = cycle_engine(t_max);
    for t in 0..=t_max {
        engine.generate_movement_clauses(t);
        engine.generate_help_clauses(t);
    }

    let clauses = (0..=t_max)
        .flat_map(|t| engine.generate_interdependence_clauses(t, 2))
        .collect::<Vec<_>>();
    assert!(!clauses.is_empty());
    assert!(clauses.iter().flatten().all(|literal| {
        matches!(
            engine.pool.key(literal.unsigned_abs() as i32),
            Some(VarKey::InterdependenceProgress { .. }) | Some(VarKey::Help { .. })
        )
    }));
}

/// Completed interdependence patterns are blocked directly without final progress variables.
#[test]
fn interdependence_clauses_omit_final_progress_states() {
    let t_max = 6;
    let order = 2;
    let mut engine = cycle_engine(t_max);
    let patterns = engine.interdependence_patterns(order);
    for t in 0..=t_max {
        engine.generate_movement_clauses(t);
        engine.generate_help_clauses(t);
        engine.generate_interdependence_clauses(t, order);
    }

    for (pattern, definition) in patterns.iter().enumerate() {
        for t in 0..=t_max {
            assert!(!engine.pool.exists(&VarKey::InterdependenceProgress {
                order,
                pattern,
                prefix_len: definition.arcs.len(),
                t,
            }));
        }
    }
}

/// Impossible static arcs omit their transitions instead of requiring a help literal.
///
/// @ai-generated
#[test]
fn interdependence_clauses_tolerate_missing_help_literals() {
    let mut engine = cycle_engine(2);
    for t in 0..=2 {
        engine.generate_movement_clauses(t);
    }
    let clauses = engine.generate_interdependence_clauses(2, 2);
    assert!(clauses.iter().all(|clause| {
        clause.iter().all(|literal| {
            !matches!(
                engine.pool.key(literal.unsigned_abs() as i32),
                Some(VarKey::Help { .. })
            )
        })
    }));
}

/// An exact order above the number of agents has no static pattern basis.
///
/// @ai-generated
#[test]
fn oversized_interdependence_order_emits_no_clauses() {
    let mut engine = cycle_engine(2);
    for t in 0..=2 {
        engine.generate_movement_clauses(t);
        engine.generate_help_clauses(t);
        assert!(engine.generate_interdependence_clauses(t, 4).is_empty());
    }
}
