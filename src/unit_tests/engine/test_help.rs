use crate::{Position, World, solver::clauses::ClauseEngine};

/// Assert that `help_support` contains exactly the expected beneficiary positions.
fn assert_help_support_keys(
    engine: &ClauseEngine,
    helper: usize,
    beneficiary: usize,
    t: usize,
    expected: &[(usize, usize)],
) {
    let support = engine.help_support(helper, beneficiary, t);
    for &(i, j) in expected {
        let pos = Position { i, j };
        assert!(
            support.contains_key(&pos),
            "expected help support at {pos:?} for t={t}; got {:?}",
            support.keys().collect::<Vec<_>>()
        );
    }
    assert_eq!(
        support.len(),
        expected.len(),
        "unexpected support size at t={t}"
    );
}

#[test]
fn test_help_support() {
    let world = World::try_from(
        "
 @  S0 S1
L0E  .  .
 @   X  X
",
    )
    .unwrap();
    let mut engine = ClauseEngine::new(&world, 5);
    for i in 0..=5 {
        engine.ctx.update(i);
    }
    assert!(engine.help_support(0, 1, 0).is_empty());
    assert_eq!(engine.help_support(0, 1, 1).len(), 1);
    assert_eq!(engine.help_support(0, 1, 2).len(), 1);
    assert_eq!(engine.help_support(0, 1, 3).len(), 1);
    assert_eq!(engine.help_support(0, 1, 4).len(), 1);
    assert!(engine.help_support(0, 1, 5).is_empty());
}

#[test]
fn test_help_support_two_lasers() {
    let world = World::try_from(
        "
 @  S0 S1
L0E  .  .
L0E  .  .
 @   X  X
",
    )
    .unwrap();
    let mut engine = ClauseEngine::new(&world, 5);
    for i in 0..=5 {
        engine.ctx.update(i);
        for (_, blocker_positions) in engine.help_support(0, 1, i) {
            assert_eq!(
                blocker_positions.size(),
                1,
                "For every helped position, there should be one single position where the helper can be located."
            );
        }
    }
    assert!(engine.help_support(0, 1, 0).is_empty());
    assert_eq!(engine.help_support(0, 1, 1).len(), 1);
    assert_eq!(engine.help_support(0, 1, 2).len(), 2);
    assert_eq!(engine.help_support(0, 1, 3).len(), 2);
    assert_eq!(engine.help_support(0, 1, 4).len(), 1);
    assert!(engine.help_support(0, 1, 5).is_empty());
}

/// Crossing laser sources should expose the exact beneficiary positions that can be helped at each
/// time step.
#[test]
fn test_help_support_two_crossing_lasers() {
    let world = World::try_from(
        "
 @  S0 S1 L0S @
L0E  .  .  .  X
 @   @  .  .  X
 @   @  .  .  @
",
    )
    .unwrap();
    let t_max = 7;
    let mut engine = ClauseEngine::new(&world, t_max);
    for i in 0..=t_max {
        engine.ctx.update(i);
    }
    assert_help_support_keys(&engine, 0, 1, 0, &[]);
    assert_help_support_keys(&engine, 0, 1, 1, &[(1, 2)]);
    assert_help_support_keys(&engine, 0, 1, 2, &[(1, 2)]);
    assert_help_support_keys(&engine, 0, 1, 3, &[(1, 2), (2, 3)]);
    assert_help_support_keys(&engine, 0, 1, 4, &[(1, 2), (2, 3), (3, 3)]);
    assert_help_support_keys(&engine, 0, 1, 5, &[(2, 3), (3, 3)]);
    assert_help_support_keys(&engine, 0, 1, 6, &[(2, 3)]);
    assert_help_support_keys(&engine, 0, 1, 7, &[]);
}

/// Crossing laser sources should expose the exact beneficiary positions that can be helped at each
/// time step.
#[test]
fn test_help_support_two_crossing_lasers_of_different_colours() {
    let world = World::try_from(
        "
 @  S0 S1 L1S @
L0E  .  .  .  @
 @   @  .  .  X
 @   @  .  .  X
",
    )
    .unwrap();
    let t_max = 7;
    let mut engine = ClauseEngine::new(&world, t_max);
    for i in 0..=t_max {
        engine.ctx.update(i);
    }
    // t=0
    assert!(engine.help_clauses(0).is_empty());
    // t=1
    assert_help_support_keys(&engine, 0, 1, 1, &[(1, 2)]);
    assert_help_support_keys(&engine, 1, 0, 1, &[]);
    // t=2
    assert_help_support_keys(&engine, 0, 1, 2, &[(1, 2), (1, 3)]);
    assert_help_support_keys(&engine, 1, 0, 2, &[]);
    // t=3
    assert_help_support_keys(&engine, 0, 1, 3, &[(1, 2), (1, 3)]);
    assert_help_support_keys(&engine, 1, 0, 3, &[]);
    // t=4
    assert_help_support_keys(&engine, 0, 1, 4, &[(1, 3)]);
    assert_help_support_keys(&engine, 1, 0, 4, &[(2, 3)]);
    // t=5
    assert_help_support_keys(&engine, 0, 1, 5, &[]);
    assert_help_support_keys(&engine, 1, 0, 5, &[(2, 3), (3, 3)]);
    // t=6
    assert_help_support_keys(&engine, 0, 1, 5, &[]);
    assert_help_support_keys(&engine, 1, 0, 5, &[(2, 3), (3, 3)]);
    // t=7
    assert!(engine.help_clauses(7).is_empty());
}
