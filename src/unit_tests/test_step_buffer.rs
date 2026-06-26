use crate::World;
use crate::solver::clauses::engine::ClauseEngine;

use super::StepBuffer;

/// Build a tiny world where extending the horizon creates additional movement variables.
fn tiny_engine() -> ClauseEngine {
    let world = World::try_from("S0 . . X").expect("failed to build test world");
    ClauseEngine::new(&world, 3)
}

/// `gather_until` must populate the requested prefix and create movement variables for that prefix.
#[test]
fn gather_until_generates_each_step_once() {
    let mut engine = tiny_engine();
    let mut buffer = StepBuffer::new(ClauseEngine::generate_movement_clauses, 5);

    let gathered = buffer.gather_until(&mut engine, 3).collect::<Vec<_>>();
    assert!(
        !gathered.is_empty(),
        "movement generation through the buffer should yield clauses"
    );
    assert!(
        engine.n_vars() == 4,
        "generating a prefix should create SAT variables in the engine"
    );
}

/// Re-asking for an already-cached prefix must not mint any new variables; extending the horizon
/// must mint more.
///
/// @ai-generated
#[test]
fn gather_until_reuses_cached_steps() {
    let mut engine = tiny_engine();
    let mut buffer = StepBuffer::new(ClauseEngine::generate_movement_clauses, 5);

    let first = buffer.gather_until(&mut engine, 1).collect::<Vec<_>>();
    let n_vars_after_first = engine.n_vars();

    let second = buffer.gather_until(&mut engine, 1).collect::<Vec<_>>();
    let n_vars_after_second = engine.n_vars();

    assert_eq!(first, second, "cached prefix should be returned unchanged");
    assert_eq!(
        n_vars_after_first, n_vars_after_second,
        "reusing a cached prefix must not mint new variables"
    );

    let _ = buffer.gather_until(&mut engine, 3).collect::<Vec<_>>();
    assert!(
        engine.n_vars() > n_vars_after_second,
        "extending the horizon should mint variables for the new steps only"
    );
}
