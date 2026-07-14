use crate::{
    Position, World,
    solver::{Clause, VarKey, clauses::ClauseEngine},
};

fn help_clauses_per_step(world: &World, t_max: usize) -> (ClauseEngine, Vec<Vec<Clause>>) {
    let mut engine = ClauseEngine::new(world, t_max);
    for t in 0..=t_max {
        engine.generate_movement_clauses(t);
    }
    let per_step = (0..=t_max)
        .map(|t| engine.generate_help_clauses(t))
        .collect();
    (engine, per_step)
}

/// Whether a `Help(helper, beneficiary, t)` variable was materialized.
fn help_exists(engine: &ClauseEngine, helper: usize, beneficiary: usize, t: usize) -> bool {
    engine.exists(&VarKey::Help {
        helper,
        beneficiary,
        t,
    })
}

/// Return the SAT literal of a `Help` variable, panicking if it was never created.
fn help_literal(engine: &ClauseEngine, helper: usize, beneficiary: usize, t: usize) -> i32 {
    engine
        .literal(&VarKey::Help {
            helper,
            beneficiary,
            t,
        })
        .unwrap_or_else(|| panic!("help({helper},{beneficiary},{t}) should exist"))
}

/// Extract the beneficiary beam positions of the `help -> OR(...)` disjunction clause.
///
/// The disjunction is the unique clause containing `-help`; the binary `pos -> help` implications
/// carry `+help` instead. This also checks that the clause is well-formed: its only negative literal
/// is `-help`, and every positive literal is an agent variable of the `beneficiary` at time `t`.
fn help_beam_positions(
    engine: &ClauseEngine,
    clauses: &[Clause],
    helper: usize,
    beneficiary: usize,
    t: usize,
) -> Vec<Position> {
    let help = help_literal(engine, helper, beneficiary, t);
    let disjunctions: Vec<&Clause> = clauses
        .iter()
        .filter(|clause| clause.contains(&-help))
        .collect();
    assert_eq!(
        disjunctions.len(),
        1,
        "there must be exactly one help -> OR(...) clause for help({helper},{beneficiary},{t})"
    );
    let disjunction = disjunctions[0];
    let mut positions = vec![];
    let mut negatives = 0;
    for &lit in disjunction {
        if lit < 0 {
            negatives += 1;
            assert_eq!(lit, -help, "the only negative literal must be -help");
            continue;
        }
        match engine.pool.key(lit) {
            Some(VarKey::Agent {
                agent_id,
                pos,
                t: at,
            }) => {
                assert_eq!(
                    agent_id, beneficiary,
                    "disjunction must be over the beneficiary"
                );
                assert_eq!(at, t, "disjunction literals must be at time {t}");
                positions.push(pos);
            }
            other => panic!("unexpected literal {lit} ({other:?}) in help disjunction"),
        }
    }
    assert_eq!(
        negatives, 1,
        "disjunction must contain exactly one -help literal"
    );
    positions.sort_unstable_by_key(|p| (p.i, p.j));
    positions
}

/// Assert that `help(helper, beneficiary, t)` exists and reifies **exactly** the expected beam
/// positions, verifying both encoding directions:
/// - `help -> OR(positions)` (the disjunction clause), and
/// - `agent(beneficiary, pos, t) -> help` (a binary implication per position).
fn assert_help(
    engine: &ClauseEngine,
    clauses: &[Clause],
    helper: usize,
    beneficiary: usize,
    t: usize,
    expected: &[(usize, usize)],
) {
    let mut expected: Vec<Position> = expected.iter().map(|&(i, j)| Position { i, j }).collect();
    expected.sort_unstable_by_key(|p| (p.i, p.j));
    let got = help_beam_positions(engine, clauses, helper, beneficiary, t);
    assert_eq!(
        got, expected,
        "help({helper},{beneficiary},{t}) beam positions mismatch"
    );

    let help = help_literal(engine, helper, beneficiary, t);
    for pos in &expected {
        let agent = engine
            .literal(&VarKey::agent(beneficiary, *pos, t))
            .expect("beneficiary agent variable must exist");
        assert!(
            clauses.iter().any(|clause| clause.len() == 2
                && clause.contains(&-agent)
                && clause.contains(&help)),
            "missing implication agent({beneficiary},{pos:?},{t}) -> help({helper},{beneficiary},{t})"
        );
    }
}

/// Assert that no `Help(helper, beneficiary, t)` variable was created — the help event is
/// geometrically impossible, so it must not appear in the pool.
fn assert_no_help(engine: &ClauseEngine, helper: usize, beneficiary: usize, t: usize) {
    assert!(
        !help_exists(engine, helper, beneficiary, t),
        "help({helper},{beneficiary},{t}) must not be materialized"
    );
}

/// Assert that no help variable exists for the pair `(helper, beneficiary)` at any time step. This
/// catches "impossible help": e.g. a helper that owns no laser can never help anyone.
fn assert_never_helps(engine: &ClauseEngine, helper: usize, beneficiary: usize, t_max: usize) {
    for t in 0..=t_max {
        assert_no_help(engine, helper, beneficiary, t);
    }
}

/// A single downstream beam tile is helpable while the beneficiary can both reach it and still make
/// the exit; no help exists at the endpoints of the horizon. Agent 1 owns no laser, so it can never
/// help agent 0.
#[test]
fn test_help_single_laser() {
    let world = World::try_from(
        "
 @  S0 S1
L0E  .  .
 @   X  X
",
    )
    .unwrap();
    let t_max = 5;
    let (engine, clauses) = help_clauses_per_step(&world, t_max);

    assert_no_help(&engine, 0, 1, 0);
    for (t, clause) in clauses.iter().enumerate().skip(1).take(4) {
        assert_help(&engine, clause, 0, 1, t, &[(1, 2)]);
    }
    assert_no_help(&engine, 0, 1, 5);
    // Agent 1 owns no laser: no impossible help must be generated in the reverse direction.
    assert_never_helps(&engine, 1, 0, t_max);
}

/// Two beams of the same colour are pooled into a **single** help variable per beneficiary, so its
/// disjunction can list several beam tiles at once.
#[test]
fn test_help_two_lasers() {
    let world = World::try_from(
        "
 @  S0 S1
L0E  .  .
L0E  .  .
 @   X  X
",
    )
    .unwrap();
    let t_max = 5;
    let (engine, clauses) = help_clauses_per_step(&world, t_max);

    assert_no_help(&engine, 0, 1, 0);
    assert_help(&engine, &clauses[1], 0, 1, 1, &[(1, 2)]);
    assert_help(&engine, &clauses[2], 0, 1, 2, &[(1, 2), (2, 2)]);
    assert_help(&engine, &clauses[3], 0, 1, 3, &[(1, 2), (2, 2)]);
    assert_help(&engine, &clauses[4], 0, 1, 4, &[(2, 2)]);
    assert_no_help(&engine, 0, 1, 5);
    assert_never_helps(&engine, 1, 0, t_max);
}

/// Two crossing beams of the **same** colour are pooled into one help variable per beneficiary and
/// expose the exact beneficiary positions that can be helped at each time step.
#[test]
fn test_help_two_crossing_lasers() {
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
    let (engine, clauses) = help_clauses_per_step(&world, t_max);

    assert_no_help(&engine, 0, 1, 0);
    assert_help(&engine, &clauses[1], 0, 1, 1, &[(1, 2)]);
    assert_help(&engine, &clauses[2], 0, 1, 2, &[(1, 2)]);
    assert_help(&engine, &clauses[3], 0, 1, 3, &[(1, 2), (2, 3)]);
    assert_help(&engine, &clauses[4], 0, 1, 4, &[(1, 2), (2, 3), (3, 3)]);
    assert_help(&engine, &clauses[5], 0, 1, 5, &[(2, 3), (3, 3)]);
    assert_help(&engine, &clauses[6], 0, 1, 6, &[(2, 3)]);
    assert_no_help(&engine, 0, 1, 7);
    assert_never_helps(&engine, 1, 0, t_max);
}

/// With crossing beams of **different** colours, each agent can help the other; the help variables
/// are directed and only expose that agent's own beam tiles.
#[test]
fn test_help_two_crossing_lasers_of_different_colours() {
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
    let (engine, clauses) = help_clauses_per_step(&world, t_max);

    // Agent 0 (L0E) helps agent 1 on the horizontal beam.
    assert_no_help(&engine, 0, 1, 0);
    assert_help(&engine, &clauses[1], 0, 1, 1, &[(1, 2)]);
    assert_help(&engine, &clauses[2], 0, 1, 2, &[(1, 2), (1, 3)]);
    assert_help(&engine, &clauses[3], 0, 1, 3, &[(1, 2), (1, 3)]);
    assert_help(&engine, &clauses[4], 0, 1, 4, &[(1, 3)]);
    for t in 5..=7 {
        assert_no_help(&engine, 0, 1, t);
    }

    // Agent 1 (L1S) helps agent 0 on the vertical beam.
    for t in 0..=3 {
        assert_no_help(&engine, 1, 0, t);
    }
    assert_help(&engine, &clauses[4], 1, 0, 4, &[(2, 3)]);
    assert_help(&engine, &clauses[5], 1, 0, 5, &[(2, 3), (3, 3)]);
    assert_help(&engine, &clauses[6], 1, 0, 6, &[(3, 3)]);
    assert_no_help(&engine, 1, 0, 7);
}

#[test]
fn number_of_is_helped_clauses_zero() {
    let w1 = World::try_from("S0 . X").unwrap();
    let w2 = World::try_from(
        "
    S0 . X
    S1 . X",
    )
    .unwrap();
    let t_max = 10;
    let mut ce = ClauseEngine::new(&w1, t_max);
    ce.generate_help_clauses(t_max);
    for t in 0..=t_max {
        ce.generate_movement_clauses(t);
        ce.generate_laser_clauses(t);
        ce.generate_help_clauses(t);
    }
    assert!(ce.generate_is_helped(t_max).is_empty());
    assert!(!ce.pool.exists(&VarKey::IsHelped {
        beneficiary: 0,
        horizon: t_max
    }));
    assert!(!ce.pool.exists(&VarKey::IsHelped {
        beneficiary: 1,
        horizon: t_max
    }));
    let mut ce = ClauseEngine::new(&w2, t_max);
    for t in 0..=t_max {
        ce.generate_movement_clauses(t);
        ce.generate_laser_clauses(t);
        ce.generate_help_clauses(t);
    }
    assert!(ce.generate_is_helped(t_max).is_empty());
    assert!(!ce.pool.exists(&VarKey::IsHelped {
        beneficiary: 0,
        horizon: t_max
    }));
    assert!(!ce.pool.exists(&VarKey::IsHelped {
        beneficiary: 1,
        horizon: t_max
    }));
}

#[test]
fn number_of_is_helped_clauses_one_laser() {
    let w = World::try_from(
        "
    @  L0S @
    S0  .  X
    S1  .  X",
    )
    .unwrap();
    let t_max = 10;
    let mut ce = ClauseEngine::new(&w, t_max);
    for t in 0..=t_max {
        ce.generate_movement_clauses(t);
        ce.generate_laser_clauses(t);
        ce.generate_help_clauses(t);
    }
    ce.generate_is_helped(t_max);
    // Cannot help at t=0 and t=t_max
    assert!(!ce.pool.exists(&VarKey::IsHelped {
        beneficiary: 0,
        horizon: t_max
    }));
    assert!(ce.pool.exists(&VarKey::IsHelped {
        beneficiary: 1,
        horizon: t_max
    }));
}

#[test]
fn number_of_is_helped_clauses_one_laser_two_laser_tiles() {
    let w = World::try_from(
        "
    @  L0S @
    S0  .  X
    .   .  .
    S1  .  X
    ",
    )
    .unwrap();
    let t_max = 10;
    let mut ce = ClauseEngine::new(&w, t_max);
    for t in 0..=t_max {
        ce.generate_movement_clauses(t);
        ce.generate_laser_clauses(t);
        ce.generate_help_clauses(t);
    }
    ce.generate_is_helped(10);
    assert!(!ce.pool.exists(&VarKey::IsHelped {
        beneficiary: 0,
        horizon: t_max
    }));
    assert!(ce.pool.exists(&VarKey::IsHelped {
        beneficiary: 1,
        horizon: t_max
    }));
}
