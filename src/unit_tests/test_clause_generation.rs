use std::collections::HashSet;

use crate::Position;
use crate::World;
use crate::solver::Clause;
use crate::solver::Literal;
use crate::solver::clauses::ClauseEngine;
use crate::solver::{ClauseGenerator, SolveMode, VarKey};
use rstest::rstest;
use rstest_reuse::{self, apply, template};

fn pos(i: usize, j: usize) -> Position {
    Position { i, j }
}

fn build(map: &str, t_max: usize) -> ClauseGenerator {
    let world = World::try_from(map).expect("failed to parse world");
    ClauseGenerator::new(&world, t_max)
}

fn check_n_possible_positions(cg: &ClauseGenerator, agent: usize, t: usize, expected: usize) {
    let all_positions = (0..=2)
        .flat_map(|i| (0..=1).map(move |j| pos(i, j)))
        .collect::<Vec<_>>();
    let mut n_available = 0;
    for pos in all_positions {
        if cg.exists(&VarKey::agent(agent, pos, t)) {
            n_available += 1;
        }
    }
    assert_eq!(n_available, expected);
}

#[template]
#[rstest]
fn standard_levels(#[values(1, 2, 3, 4, 5, 6)] level: usize) {}

// ─── initialization ──────────────────────────────────────────────────────────

#[test]
fn test_position_validity_single_agent() {
    let t_max = 4;
    let world = World::try_from("S0 . X").expect("Failed to parse world");
    let mut generator = ClauseGenerator::new(&world, t_max);
    generator.generate(t_max, SolveMode::Standard, false);

    let start_pos = world.starts()[0];
    // Pos (0,0) exists at t={0,1,2}
    for t in 0..=2 {
        assert!(generator.exists(&VarKey::agent(0, start_pos, t)));
    }
    // Pos (0, 0) does not exist a t={3,4} (because it is too far from the exit)
    assert!(!generator.exists(&VarKey::agent(0, start_pos, 3)));
    assert!(!generator.exists(&VarKey::agent(0, start_pos, 4)));
    // Pos (0,1) exists at t={1, 2, 3}
    assert!(!generator.exists(&VarKey::agent(0, pos(0, 1), 0)));
    for t in 1..=3 {
        assert!(generator.exists(&VarKey::agent(0, pos(0, 1), t)));
    }
    assert!(!generator.exists(&VarKey::agent(0, pos(0, 1), 4)));
    // Pos (0, 2) exists at t={2, 3, 4}
    assert!(!generator.exists(&VarKey::agent(0, pos(0, 2), 0)));
    assert!(!generator.exists(&VarKey::agent(0, pos(0, 2), 1)));
    for t in 2..=4 {
        assert!(generator.exists(&VarKey::agent(0, pos(0, 2), t)));
    }
}

#[test]
fn possible_positions_multiple_agents() {
    let world = World::try_from(
        "
        S0 S1
         .  .
         X  X ",
    )
    .expect("Failed to parse world");
    let t_max = 10;
    let mut cg = ClauseGenerator::new(&world, t_max);

    let (clauses, assumptions) = cg.generate(t_max, SolveMode::Standard, false);
    assert!(!clauses.is_empty());
    assert!(assumptions.is_empty());
    // t=0, only start position
    for agent in 0..=1 {
        let start = world.starts()[agent];
        assert!(cg.exists(&VarKey::agent(agent, start, 0)));
        check_n_possible_positions(&cg, agent, 0, 1);
    }

    // t=1: two possible positions (initial and south only).
    // Opt 2: the east/west neighbour is the other agent's start, so it is pruned.
    check_n_possible_positions(&cg, 0, 1, 2);
    check_n_possible_positions(&cg, 1, 1, 2);
    // t=2: 5 possible positions (all except the furthest exit)
    check_n_possible_positions(&cg, 0, 2, 5);
    check_n_possible_positions(&cg, 1, 2, 5);
    // From t up to t_max - 3, every position should be possible
    for t in 3..=(t_max - 2) {
        check_n_possible_positions(&cg, 0, t, 6);
        check_n_possible_positions(&cg, 1, t, 6);
    }
    // At t=t_max-1,, 4 positions should be available (the exits and the tiles at one step)
    check_n_possible_positions(&cg, 0, t_max - 1, 4);
    check_n_possible_positions(&cg, 1, t_max - 1, 4);
    // At t=t_max, only the exit positions
    let exits = world.exits_positions();
    for agent in 0..=1 {
        for e in exits.clone() {
            assert!(cg.exists(&VarKey::agent(agent, e, t_max)));
        }
        check_n_possible_positions(&cg, agent, t_max, 2);
    }
}

#[test]
fn test_at_most_one_position_clause_structure() {
    let mut cg = build("S0 . X", 10);
    let (clauses, _) = cg.generate(1, SolveMode::Standard, false);

    let a00 = cg
        .literal(&VarKey::agent(0, pos(0, 0), 1))
        .expect("agent(0,(0,0),1)");
    let a01 = cg
        .literal(&VarKey::agent(0, pos(0, 1), 1))
        .expect("agent(0,(0,1),1)");

    // At-most-one (pairwise): {-a00, -a01} must appear.
    assert!(
        clauses
            .iter()
            .any(|c| c.len() == 2 && c.contains(&-a00) && c.contains(&-a01)),
        "at-most-one clause [-a00, -a01] missing"
    );
}

/// Agent at (0,1) at t=1 must have come from (0,0) at t=0 (the only predecessor).
/// The clause `[-cur, prev]` encodes this.
#[test]
fn test_time_wise_adjacency_clause_structure() {
    let mut cg = build("S0 . X", 10);
    let (clauses, _) = cg.generate(1, SolveMode::Standard, false);

    let cur = cg
        .literal(&VarKey::agent(0, pos(0, 1), 1))
        .expect("agent(0,(0,1),1)");
    let prev = cg
        .literal(&VarKey::agent(0, pos(0, 0), 0))
        .expect("agent(0,(0,0),0)");

    assert!(
        clauses
            .iter()
            .any(|c| c.len() == 2 && c.contains(&-cur) && c.contains(&prev)),
        "adjacency clause [-cur, prev] for (0,1) at t=1 must exist"
    );
}

/// Two agents sharing a reachable position must have a binary exclusion clause `[-v0, -v1]`.
/// Such clause must not exist for tiles that entail no overlap for a given t.
#[test]
fn test_no_overlap_binary_exclusion_clause() {
    // World: "S0 . S1 X X" — agent 0 at (0,0), agent 1 at (0,2), exits at (0,3)/(0,4).
    // At t=1:
    //   agent 0 reachable: {(0,0), (0,1)}         (stay or step east)
    //   agent 1 reachable: {(0,1), (0,2), (0,3)}  (step west, stay, or step east to exit)
    //   shared: {(0,1)}  → exclusion clause must exist
    //   exclusive to agent 0: {(0,0)}  → no exclusion clause
    //   exclusive to agent 1: {(0,2), (0,3)}  → no exclusion clause
    let mut cg = build("S0 . S1 X X", 5);
    let (clauses, _) = cg.generate(1, SolveMode::Standard, false);

    // ── shared position (0,1): exclusion clause must exist ──────────────────
    let v0_01 = cg
        .literal(&VarKey::agent(0, pos(0, 1), 1))
        .expect("agent 0 at (0,1) t=1");
    let v1_01 = cg
        .literal(&VarKey::agent(1, pos(0, 1), 1))
        .expect("agent 1 at (0,1) t=1");
    assert!(
        clauses
            .iter()
            .any(|c| c.len() == 2 && c.contains(&-v0_01) && c.contains(&-v1_01)),
        "no-overlap binary clause [-v0, -v1] at shared (0,1) t=1 must exist"
    );

    // ── exclusive-to-agent-0 position (0,0): no exclusion clause ────────────
    // Agent 1 cannot reach (0,0) from (0,2) in one step, so it has no variable there.
    assert!(
        !cg.exists(&VarKey::agent(1, pos(0, 0), 1)),
        "agent 1 must have no variable at agent-0-exclusive (0,0) t=1"
    );
    // Directly: no 2-literal clause pairs -v0_at_00 with any agent-1 variable.
    let v0_00 = cg
        .literal(&VarKey::agent(0, pos(0, 0), 1))
        .expect("agent 0 at (0,0) t=1");
    for p in [pos(0, 1), pos(0, 2), pos(0, 3)] {
        if let Some(v1_p) = cg.literal(&VarKey::agent(1, p, 1)) {
            assert!(
                !clauses
                    .iter()
                    .any(|c| c.len() == 2 && c.contains(&-v0_00) && c.contains(&-v1_p)),
                "no exclusion clause between agent 0 at (0,0) and agent 1 at {p:?}"
            );
        }
    }
}

// ─── no_following_conflict ───────────────────────────────────────────────────

/// Agent 1 must not enter a cell that agent 0 just vacated (swap prohibition).
/// Encoded as `[-a1@pos@t, -a0@pos@(t-1)]`.
#[test]
fn test_no_following_conflict_encodes_swap_prohibition() {
    // S0 at (0,0), S1 at (0,1).
    // Opt 2: (0,0) is S0's start, so agent 1 cannot be there at t=1 — the variable is pruned.
    let mut cg = build("S0 S1 X X", 5);
    let (_, _) = cg.generate(2, SolveMode::Standard, false);

    // Agent 1 cannot be at (0,0) at t=1 (Opt 2 prunes it entirely).
    assert!(
        !cg.exists(&VarKey::agent(1, pos(0, 0), 1)),
        "agent(1, (0,0), 1) should be pruned by Opt 2"
    );

    // The no-following-conflict clause still applies at t=2: if agent 1 is at (0,0) at t=2
    // (reached from (0,1) at t=1), then agent 0 was NOT at (0,0) at t=1.
    let a0_t1 = cg
        .literal(&VarKey::agent(0, pos(0, 0), 1))
        .expect("agent 0 at (0,0) t=1");
    let a1_t2 = cg
        .literal(&VarKey::agent(1, pos(0, 0), 2))
        .expect("agent 1 at (0,0) t=2");
    let clauses = cg.generate(2, SolveMode::Standard, false).0;
    assert!(
        clauses
            .iter()
            .any(|c| c.len() == 2 && c.contains(&-a1_t2) && c.contains(&-a0_t1)),
        "no-following-conflict clause [-a1@(0,0)@2, -a0@(0,0)@1] must exist"
    );
}

// ─── stays_on_exit ───────────────────────────────────────────────────────────

/// An agent on an exit at t-1 must stay at that exit at t: `[-prev, cur]`.
#[test]
fn test_stays_on_exit_implication_clause() {
    // "S0 . X": exit at (0,2). Agent can reach exit at t=2; at t=3 must stay.
    let mut cg = build("S0 . X", 4);
    let (clauses, _) = cg.generate(3, SolveMode::Standard, false);

    let prev = cg
        .literal(&VarKey::agent(0, pos(0, 2), 2))
        .expect("exit at t=2");
    let cur = cg
        .literal(&VarKey::agent(0, pos(0, 2), 3))
        .expect("exit at t=3");
    assert!(
        clauses
            .iter()
            .any(|c| c.len() == 2 && c.contains(&-prev) && c.contains(&cur)),
        "stays_on_exit clause [-prev, cur] must exist"
    );
}

#[apply(standard_levels)]
fn test_objective_reaches_exit(level: usize) {
    let world = World::get_level(level).expect("Failed to parse world");
    let mut generator = ClauseGenerator::new(&world, 21);

    let obj_clauses = generator.objective(21, false);
    assert_eq!(obj_clauses.len(), world.n_agents());
    for clause in obj_clauses {
        assert_eq!(clause.len(), world.n_exits());
    }
}

#[test]
fn test_objective_multiple_agents_multiple_exits() {
    let world = World::try_from("S0 S1 . .\n. . . .\nX X X X").expect("Failed to parse world");
    let mut generator = ClauseGenerator::new(&world, 10);

    let objective_clauses = generator.objective(10, false);
    assert_eq!(objective_clauses.len(), 2, "One objective clause per agent");
    for clause in &objective_clauses {
        assert!(!clause.is_empty());
    }
}

#[test]
fn test_gem_must_be_collected_clause() {
    let world = World::try_from("S0 G X").expect("Failed to parse world");
    let mut cg = ClauseEngine::new(&world, 2);
    let clauses = cg.gems_must_be_collected(2);
    assert_eq!(
        clauses.len(),
        1,
        "There should be one single time step where the agent can collect the gem."
    );

    let gem_pos = pos(0, 1);
    assert!(cg.exists(&VarKey::agent(0, gem_pos, 1)));
    assert!(
        !cg.exists(&VarKey::agent(0, gem_pos, 2)),
        "gem position should be pruned at the final horizon when it can no longer reach an exit"
    );
}

#[test]
fn test_gem_must_be_collected_clause_2agents() {
    let world = World::try_from(
        "
        S0 G  X
        .  S1 X
        ",
    )
    .expect("Failed to parse world");
    let gem_pos = pos(0, 1);
    let mut cg = ClauseEngine::new(&world, 2);
    let clauses = cg.gems_must_be_collected(2);
    assert_eq!(clauses.len(), 1, "There is one single gem");
    let a = cg.literal(&VarKey::agent(0, gem_pos, 1)).unwrap();
    let b = cg.literal(&VarKey::agent(1, gem_pos, 1)).unwrap();
    assert!(clauses.contains(&vec![a, b]) || clauses.contains(&vec![b, a]));

    for agent in 0..world.n_agents() {
        assert!(
            !cg.exists(&VarKey::agent(agent, gem_pos, 2)),
            "gem position should be pruned at the final horizon when it can no longer reach an exit"
        );
    }
}

#[test]
fn test_gem_must_be_collected_clause_2agents_longer_horizon() {
    let world = World::try_from(
        "
        S0 G  X
        .  S1 X
        ",
    )
    .expect("Failed to parse world");
    let gem_pos = pos(0, 1);
    let mut cg = ClauseEngine::new(&world, 3);
    let clauses = cg.gems_must_be_collected(3);
    assert_eq!(clauses.len(), 1, "There is one single gem !");
    // For both time steps, chack th
    for agent in 0..world.n_agents() {
        for tau in 1..3 {
            let literal = cg
                .literal(&VarKey::agent(agent, gem_pos, tau))
                .expect("literal should exist");
            assert!(clauses[0].contains(&literal));
        }
    }
}

#[test]
fn test_gem_must_be_collected_clause_3gems() {
    let world = World::try_from(
        "
        S0 G G X
        .  . . .
        .  . . .
        S1 . G X
        ",
    )
    .expect("Failed to parse world");
    let t_max = 5;
    let mut cg = ClauseEngine::new(&world, t_max);
    let clauses = cg.gems_must_be_collected(t_max);
    for c in &clauses {
        println!("{c:?}");
    }
    assert_eq!(
        clauses.len(),
        world.n_gems(),
        "There should be one clause per gem"
    );

    // Check gem 0 at (0,1) and gem 1 at (0,2): only accessible by agent 0
    // within the time window [1, 3] for gem 0 and [2, 4] for gem 1
    let t_min = [1, 2];
    let t_max = [3, 4];
    let positions = [Position { i: 0, j: 1 }, Position { i: 0, j: 2 }];
    for (i, gem_pos) in positions.into_iter().enumerate() {
        for tau in t_min[i]..=t_max[i] {
            if tau <= t_max[i] {
                let lit = cg
                    .literal(&VarKey::agent(0, gem_pos, tau))
                    .expect("literal should exist");
                assert!(clauses[i].contains(&lit));
            }
        }
        assert_eq!(
            clauses[i].len(),
            3,
            "Gem {i} should be accessible at three time steps by agent 0"
        );
    }
    // Check gem 2 at (3,2): only accessible by agent 1 within the time window [2, 4]
    let gem_pos = Position { i: 3, j: 2 };
    for tau in 2..=4 {
        let err_message = format!("literal agent=1, pos={gem_pos:?}, t={tau} should exist");
        let lit = cg
            .literal(&VarKey::agent(1, gem_pos, tau))
            .expect(&err_message);
        assert!(clauses[2].contains(&lit));
    }
    assert_eq!(
        clauses[2].len(),
        3,
        "Gem 2 should be accessible at three time steps by agent 1"
    );
}

// ─── beam_activation ─────────────────────────────────────────────────────────

/// For the first blockable beam tile, `beam_activation` encodes `active ↔ ¬agent_var`
/// as two clauses: `[-active, -agent]` and `[active, agent]`.
#[test]
fn test_beam_activation_first_tile_encodes_biconditional() {
    // L0N at (2,0) going North; beam path (1,0), (0,0). Agent 0 starts at (1,0).
    let world = World::try_from(". X\nS0 .\nL0N .").expect("Failed to parse world");
    let laser_id = world.sources().collect::<Vec<_>>()[0].1.laser_id();
    let mut cg = ClauseGenerator::new(&world, 10);
    let (clauses, _) = cg.generate(2, SolveMode::Standard, false);

    // At t=2 agent can reach (1,0): the first (and only) blockable tile.
    let active = cg.literal(&VarKey::laser(laser_id, pos(1, 0), 2));
    let agent = cg.literal(&VarKey::agent(0, pos(1, 0), 2));

    if let (Some(l), Some(a)) = (active, agent) {
        // active → ¬agent: [-l, -a]
        assert!(
            clauses
                .iter()
                .any(|c| c.len() == 2 && c.contains(&-l) && c.contains(&-a)),
            "clause [-active, -agent] for first beam tile missing"
        );
        // ¬active → agent (= active ∨ agent): [l, a]
        assert!(
            clauses
                .iter()
                .any(|c| c.len() == 2 && c.contains(&l) && c.contains(&a)),
            "clause [active, agent] for first beam tile missing"
        );
    } else {
        panic!("expected laser and agent variables at beam tile (1,0) at t=2");
    }
}

#[test]
fn test_laser_blocking_same_colour() {
    let world = World::try_from(".   X\nS0  .\nL0N .").expect("Failed to parse world");
    let mut generator = ClauseGenerator::new(&world, 10);
    let (clauses, _) = generator.generate(2, SolveMode::Standard, false);
    assert!(!clauses.is_empty());

    assert_eq!(world.sources().count(), 1);
    let (source_pos, _) = world.sources().next().unwrap();
    assert_eq!(source_pos, pos(2, 0));
}

/// Help tracking reifies `Help(h, b, t)` from the beneficiary standing on one of the helper's beam
/// tiles: each beam occupancy implies the help event, and the help event implies the beneficiary is
/// on some beam tile.
///
/// @ai-generated
#[test]
fn test_help_event_clauses_track_blocked_beam_usage() {
    let world = World::try_from(
        "
        L0E .  .  X
        S0  .  S1 X",
    )
    .expect("Failed to parse world");
    let mut cg = ClauseGenerator::new(&world, 4);
    let (clauses, _) = cg.generate(2, SolveMode::NoAsymmetricCooperation, false);

    let help = cg
        .literal(&VarKey::Help {
            helper: 0,
            beneficiary: 1,
            t: 2,
        })
        .expect("help(0,1,2) should be tracked");
    let beneficiary_on_beam = cg
        .literal(&VarKey::agent(1, pos(0, 2), 2))
        .expect("agent 1 can use the downstream beam tile");

    assert!(
        clauses
            .iter()
            .any(|c| c.len() == 2 && c.contains(&-beneficiary_on_beam) && c.contains(&help)),
        "beneficiary on helper beam must imply the help event"
    );
    assert!(
        clauses
            .iter()
            .any(|c| c.contains(&-help) && c.contains(&beneficiary_on_beam)),
        "the help event must imply the beneficiary is on one of the helper's beam tiles"
    );
}

// ─── no_step_on_active_laser ─────────────────────────────────────────────────

/// A different-colour agent on a blockable beam tile and the laser variable must be jointly
/// excluded: `[-agent, -active]`.
#[test]
fn test_no_step_on_active_laser_binary_clause() {
    // L0S at (0,0) going South; beam at (1,0). Agent 1 (colour 1) starts at (1,1) and can
    // reach (1,0) at t=1.  Agent 0 (colour 0) at (2,0) can block the beam at (1,0).
    let world = World::try_from("L0S . X\n.   S1 X\nS0  . .").expect("Failed to parse world");
    let laser_id = world.sources().next().unwrap().1.laser_id();
    let mut cg = ClauseGenerator::new(&world, 2);
    let (clauses, _) = cg.generate(1, SolveMode::Standard, false);

    let agent_1_at_10 = cg.literal(&VarKey::agent(1, pos(1, 0), 1));
    let laser_at_10 = cg.literal(&VarKey::laser(laser_id, pos(1, 0), 1));

    match (agent_1_at_10, laser_at_10) {
        (Some(a), Some(l)) => {
            assert!(
                clauses
                    .iter()
                    .any(|c| c.len() == 2 && c.contains(&-a) && c.contains(&-l)),
                "no-step clause [-agent_1, -laser] at (1,0) t=1 must exist"
            );
        }
        (Some(a), None) => {
            // Constant-active beam tile: unit clause [-agent]
            assert!(
                clauses.iter().any(|c| c.len() == 1 && c.contains(&-a)),
                "unit clause [-agent_1] for constant-active beam tile must exist"
            );
        }
        _ => {} // agent 1 cannot reach (1,0) at this horizon — test not applicable
    }
}

/// Assert that every reachable downstream beam-tile variable for agent 1 is forbidden by a unit
/// clause or an assumption across the generated horizon.
fn assert_no_cooperation_does_not_generate_beam_positions(
    cg: &ClauseGenerator,
    world: &World,
    clauses: Vec<Clause>,
    assumptions: Vec<Literal>,
) {
    let clause_literals: HashSet<i32> = clauses.into_iter().flatten().map(|l| l.abs()).collect();
    // 1) Check that no laser variable is present in the formula
    for (pos, laser) in world.lasers() {
        for t in 0..=cg.t_max() {
            let key = VarKey::Laser {
                laser_id: laser.laser_id(),
                pos,
                t,
            };
            if let Some(lit) = cg.literal(&key) {
                assert!(
                    !clause_literals.contains(&lit),
                    "{key:?} (literal={lit}) should not be among the clause literals"
                );
            }
        }
    }
    // 2) Verify that all agents are assumed not to walk on any foreign colour laser tile
    for agent_id in 0..world.n_agents() {
        for (pos, laser) in world.lasers() {
            if agent_id == laser.agent_id() {
                continue;
            }
            for t in 0..=cg.t_max() {
                let key = VarKey::Agent { agent_id, pos, t };
                if let Some(lit) = cg.literal(&key) {
                    assert!(
                        assumptions.contains(&-lit),
                        "{key:?} (literal={lit}) should be assumed to be false"
                    );
                }
            }
        }
    }
}

/// `NoCooperation` must forbid a non-owner from using downstream beam tiles over multiple time
/// steps, even when the owner could block that beam in standard mode.
#[test]
fn test_no_cooperation_forbids_non_owner_on_blockable_downstream_beam_tile() {
    let world = World::try_from(
        "
         .  S0  S1  .  .
        L0E  .   .  @  .
         .   .   .  .  .
         .   X   X  .  .
        ",
    )
    .expect("Failed to parse world");
    let t_max = 3;
    let mut cg = ClauseGenerator::new(&world, t_max);
    let (clauses, assumptions) = cg.generate(t_max, SolveMode::NoCooperation, false);
    assert_no_cooperation_does_not_generate_beam_positions(&cg, &world, clauses, assumptions);
}

/// Reusing a generator across modes must not let standard beam-activation clauses weaken
/// `NoCooperation`; downstream beam tiles must still be explicitly forbidden at every generated
/// time step.
#[test]
fn test_no_cooperation_after_standard_generation_forbids_same_downstream_beam_tile() {
    let world = World::try_from(
        "
         .  S0  S1
        L0E  .   .
         .   .   .
         .   X   X
        ",
    )
    .expect("Failed to parse world");
    let t_max = 9;
    let mut cg = ClauseGenerator::new(&world, t_max);
    let _ = cg.generate(t_max, SolveMode::Standard, false);
    let (clauses, assumptions) = cg.generate(t_max, SolveMode::NoCooperation, false);
    assert_no_cooperation_does_not_generate_beam_positions(&cg, &world, clauses, assumptions);
}

/// An unblockable beam tile (same-colour agent can never reach it) is constant-active and
/// must generate a unit clause `[-agent]` forbidding every other-colour agent from it.
#[test]
fn test_unblockable_beam_tile_generates_unit_clause() {
    // L0E at (0,0); agent 0 walled in by (1,1)=@ — cannot reach any beam tile.
    // Agent 1 at (1,2) can reach beam tile (0,2) at t=1.
    let world = World::try_from("L0E .  .  X\nS0  @  S1 X").expect("Failed to parse world");
    let laser_id = world.sources().next().unwrap().1.laser_id();
    let mut cg = ClauseGenerator::new(&world, 4);
    let (clauses, _) = cg.generate(2, SolveMode::Standard, false);

    // The beam tile (0,2) is downstream and unreachable by agent 0 → constant-active → no var.
    assert!(
        !cg.exists(&VarKey::laser(laser_id, pos(0, 2), 2)),
        "constant-active tile must not have a laser variable"
    );

    // Agent 1 that can reach (0,2) must be forbidden by a unit clause.
    let agent_1_at_02 = cg.literal(&VarKey::agent(1, pos(0, 2), 2));
    if let Some(a) = agent_1_at_02 {
        assert!(
            clauses.iter().any(|c| c.len() == 1 && c.contains(&-a)),
            "unit clause [-agent_1] for constant-active beam tile (0,2) must exist"
        );
    }
}

#[test]
fn test_laser_blocks_different_colour_agent() {
    let world = World::try_from("L0S . X\n.   S1 X\nS0  . .").expect("Failed to parse world");
    let mut generator = ClauseGenerator::new(&world, 2);
    let (clauses, _) = generator.generate(1, SolveMode::Standard, false);
    assert!(!clauses.is_empty());
    assert_eq!(world.n_agents(), 2);
    let sources: Vec<_> = world.sources().collect();
    assert_eq!(sources.len(), 1);
    assert_eq!(sources[0].1.agent_id(), 0, "Laser is colour 0");
}

#[test]
fn test_unblockable_constant_active_laser() {
    let world = World::try_from("L0E .  .  X\nS0  @  S1 X").expect("Failed to parse world");
    let mut generator = ClauseGenerator::new(&world, 4);
    let (clauses, _) = generator.generate(2, SolveMode::Standard, false);
    assert!(!clauses.is_empty());

    assert!(!generator.exists(&VarKey::Laser {
        laser_id: 0,
        pos: pos(0, 2),
        t: 2,
    }));
}

// ─── laser paths / multi-source ──────────────────────────────────────────────

/// Each facing laser stops at the other's source tile, not beyond it.
#[test]
fn test_two_lasers_stop_at_each_other() {
    // L0E at (0,0) → beam (0,1); L1W at (0,2) → beam (0,1). Neither reaches past the other's source.
    let world = World::try_from("L0E . L1W X X\nS0  . S1  . .").expect("Failed to parse world");
    let mut generator = ClauseGenerator::new(&world, 10);
    let (clauses, _) = generator.generate(2, SolveMode::Standard, false);
    assert!(!clauses.is_empty());

    let sources: Vec<_> = world.sources().collect();
    assert_eq!(sources.len(), 2);
    let (id0, id1) = (sources[0].1.laser_id(), sources[1].1.laser_id());

    // L0E must not create a laser variable at (0,2) (that is L1W's source tile).
    assert!(
        !generator.exists(&VarKey::laser(id0, pos(0, 2), 2)),
        "L0E must not extend past L1W source at (0,2)"
    );
    // L1W must not create a laser variable at (0,0) (that is L0E's source tile).
    assert!(
        !generator.exists(&VarKey::laser(id1, pos(0, 0), 2)),
        "L1W must not extend past L0E source at (0,0)"
    );
    // Both lasers share the middle tile (0,1).
    // Opt 3: (0,1) is the first tile of L0E (owner=0) AND of L1W (owner=1). Neither agent is the
    // owner of both, so (0,1) is forbidden for both agents at all times.  Both beams are therefore
    // constant-active at (0,1) and no laser variable is created for either.
    assert!(
        !generator.exists(&VarKey::laser(id0, pos(0, 1), 2)),
        "L0E beam at (0,1) should be constant-active (no variable)"
    );
    assert!(
        !generator.exists(&VarKey::laser(id1, pos(0, 1), 2)),
        "L1W beam at (0,1) should be constant-active (no variable)"
    );
}

/// Two same-colour, same-direction laser sources each keep their own independent beam variable.
#[test]
fn test_multiple_same_colour_same_direction_lasers_get_independent_beams() {
    // Two colour-0 south lasers: source 0 at (0,1), source 1 at (0,3).
    let world = World::try_from(".  L0S .  L0S .\nS0 .   .  .   S1\nX  .   .  .   X")
        .expect("Failed to parse world");
    let mut cg = ClauseGenerator::new(&world, 10);
    let (clauses, _) = cg.generate(3, SolveMode::Standard, false);
    assert!(!clauses.is_empty());

    let sources: Vec<_> = world.sources().collect();
    assert_eq!(sources.len(), 2);
    let id0 = sources[0].1.laser_id(); // source at (0,1)
    let id1 = sources[1].1.laser_id(); // source at (0,3)

    // Source 0 owns (1,1); source 1 owns (1,3). Neither owns the other's column.
    assert!(
        cg.exists(&VarKey::laser(id0, pos(1, 1), 3)),
        "source 0 must have variable at its beam tile (1,1)"
    );
    assert!(
        !cg.exists(&VarKey::laser(id0, pos(1, 3), 3)),
        "source 0 must not have a variable at source 1's beam tile (1,3)"
    );
    assert!(
        cg.exists(&VarKey::laser(id1, pos(1, 3), 3)),
        "source 1 must have variable at its beam tile (1,3)"
    );
    assert!(
        !cg.exists(&VarKey::laser(id1, pos(1, 1), 3)),
        "source 1 must not have a variable at source 0's beam tile (1,1)"
    );
    // The two variables are distinct literals.
    let lit0 = cg.literal(&VarKey::laser(id0, pos(1, 1), 3)).unwrap();
    let lit1 = cg.literal(&VarKey::laser(id1, pos(1, 3), 3)).unwrap();
    assert_ne!(lit0, lit1, "independent beams must have distinct literals");
}

// ─── reachability pruning optimizations ─────────────────────────────────────

/// Opt 2 — Start tile pruning.
///
/// At t=1, no agent may occupy another agent's t=0 start position: the
/// no-following-conflict rule already makes such an assignment unsatisfiable,
/// so the variable can be pruned from the reachable set.
///
/// World:  S0 S1 X X  (1 × 4 grid)
///   (0,0)=S0, (0,1)=S1, (0,2)=X, (0,3)=X
///
/// Expected at t=1:
///   • agent(1, (0,0), 1) — agent 1 at S0's start — does NOT exist (pruned)
///   • agent(0, (0,1), 1) — agent 0 at S1's start — does NOT exist (pruned)
///
/// Expected at t=2 (restriction is t=1 only):
///   • agent(1, (0,0), 2) DOES exist  (agent 1 can reach (0,0) from (0,1))
///   • agent(0, (0,1), 2) DOES exist  (agent 0 can reach (0,1) from (0,0))
#[test]
fn test_opt2_start_tile_pruned_at_t1_only() {
    let mut cg = build("S0 S1 X X", 5);
    cg.generate(2, SolveMode::Standard, false);

    // At t=1: each agent's other-agent start is pruned.
    assert!(
        !cg.exists(&VarKey::agent(1, pos(0, 0), 1)),
        "agent(1, S0-start, t=1) should be pruned"
    );
    assert!(
        !cg.exists(&VarKey::agent(0, pos(0, 1), 1)),
        "agent(0, S1-start, t=1) should be pruned"
    );

    // At t=2: the pruning no longer applies; both positions become reachable again.
    assert!(
        cg.exists(&VarKey::agent(1, pos(0, 0), 2)),
        "agent(1, S0-start, t=2) must exist — Opt 2 pruning is t=1 only"
    );
    assert!(
        cg.exists(&VarKey::agent(0, pos(0, 1), 2)),
        "agent(0, S1-start, t=2) must exist — Opt 2 pruning is t=1 only"
    );
}

/// Opt 3 — First beam tile pruning.
///
/// A non-owner agent can never safely stand on the first tile of a laser beam:
/// if the owner can reach it, beam-active ↔ ¬owner, so the non-owner would require
/// the owner present — impossible by no_overlap.  If the owner cannot reach it the
/// beam is constant-active and the non-owner dies.  Either way the variable is pruned.
///
/// World:
///   L0S .  X     row 0  — L0S source at (0,0) fires south
///   .   S1 X     row 1  — first beam tile (1,0); S1 at (1,1), one step west of it
///   S0  .  .     row 2  — S0 at (2,0), one step north of the first beam tile
///
/// Expected:
///   • agent(1, (1,0), t) does NOT exist for any t  (non-owner at first beam tile)
///   • agent(0, (1,0), 1) DOES exist               (owner may stand on first beam tile)
///   • laser(laser_id, (1,0), 1) DOES exist        (beam-activation variable created
///                                                   because owner can reach the tile)
#[test]
fn test_opt3_first_beam_tile_pruned_for_non_owner() {
    let world = World::try_from("L0S . X\n. S1 X\nS0 . .").expect("Failed to parse world");
    let laser_id = world.sources().next().unwrap().1.laser_id();
    let mut cg = ClauseGenerator::new(&world, 10);
    cg.generate(3, SolveMode::Standard, false);

    // Non-owner (agent 1) must never have a variable at the first beam tile (1,0).
    for t in 0..=3 {
        assert!(
            !cg.exists(&VarKey::agent(1, pos(1, 0), t)),
            "agent(1, first-beam-tile, t={t}) must be pruned by Opt 3"
        );
    }

    // Owner (agent 0) can still reach the first beam tile and has a variable there.
    assert!(
        cg.exists(&VarKey::agent(0, pos(1, 0), 1)),
        "agent(0, first-beam-tile, t=1) must exist — owner is not pruned"
    );

    // Because the owner can reach (1,0), beam activation creates a laser variable there.
    assert!(
        cg.exists(&VarKey::laser(laser_id, pos(1, 0), 1)),
        "laser variable at first beam tile t=1 must exist when owner can reach it"
    );
}

/// When two beams of the same colour cross, each crossing cell carries one variable per
/// laser source, not a single shared variable.
#[test]
fn test_crossing_lasers_keep_independent_variables() {
    // colour-0 south lasers at (0,1) and (0,2); colour-0 east laser at (1,0).
    // The east beam crosses both south beams at (1,1) and (1,2).
    let world = World::try_from(
        ".   L0S L0S  X
        L0E  .   .    .
        S0   .   .    .
        S1   .   .    .
        .    .   .    X",
    )
    .expect("Failed to parse world");
    let mut cg = ClauseGenerator::new(&world, 20);
    let (clauses, _) = cg.generate(10, SolveMode::Standard, false);
    assert!(!clauses.is_empty());

    // At each crossing position there must be at least two distinct laser variables
    // (one per source whose beam passes through that cell).
    let sources: Vec<_> = world.sources().collect();
    for (src_pos, src) in &sources {
        let id = src.laser_id();
        // Verify the variable for this source at an expected crossing position exists and is unique.
        for other in sources.iter().filter(|(op, _)| op != src_pos) {
            let other_id = other.1.laser_id();
            // If both own variables at (1,1), they must be distinct.
            if cg.exists(&VarKey::laser(id, pos(1, 1), 10))
                && cg.exists(&VarKey::laser(other_id, pos(1, 1), 10))
            {
                let l_self = cg.literal(&VarKey::laser(id, pos(1, 1), 10)).unwrap();
                let l_other = cg.literal(&VarKey::laser(other_id, pos(1, 1), 10)).unwrap();
                assert_ne!(
                    l_self, l_other,
                    "crossing beams at (1,1) must have distinct literals"
                );
            }
        }
    }
}

/// Every agent position variable that exists at time `t` must be governed by
/// `exactly_one_position`, i.e. its position must belong to the agent's relevant set at `t`.
///
/// A variable created outside the relevant set is a "phantom": it is constrained neither by
/// `exactly_one_position` nor by `time_wise_adjacency`, so the SAT solver may freely set it
/// true. The decoder then sees the agent at two positions at the same step and may pick the
/// phantom, yielding an impossible (e.g. diagonal) move in the returned plan.
///
/// @ai-generated
#[test]
fn test_no_phantom_agent_variables() {
    // Two exits, two agents and two length-1 beams: forced-exit and laser relevance pruning both
    // shrink agents' relevant sets as `t` grows, which previously left phantom exit variables.
    let world = World::try_from(
        "
S0 S1 . . .
L0E . . @ .
L1E . . @ .
X X . @ .
. . . . .",
    )
    .unwrap();
    let t_max = 13;
    let mut cg = ClauseGenerator::new(&world, t_max);
    // Drive the incremental solve exactly like the Python solver: generate at every horizon.
    for t in cg.solution_lower_bound()..=t_max {
        cg.generate(t, SolveMode::Standard, false);
    }
    for agent in 0..2 {
        for t in 0..=t_max {
            for i in 0..5 {
                for j in 0..5 {
                    let p = pos(i, j);
                    if cg.exists(&VarKey::agent(agent, p, t)) {
                        assert!(
                            cg.engine
                                .ctx
                                .relevant_positions_for_agent(agent, t)
                                .contains(&p),
                            "phantom agent variable: agent={agent} pos={p:?} t={t} exists but is \
                             not in the agent's relevant set, so it escapes exactly_one_position"
                        );
                    }
                }
            }
        }
    }
}

/// Help clauses must include binary implications from beneficiary beam occupancy into the
/// corresponding concrete `Help(helper, beneficiary, t)` literal.
///
/// @ai-generated
#[test]
fn help_clauses_include_beneficiary_to_help_implications() {
    let world = World::try_from(
        "
     S0 . . S1
    L0E . . .
     .  . . L1W
     X  . . X",
    )
    .expect("failed to parse world");
    let mut cg = ClauseGenerator::new(&world, 10);
    let (clauses, _) = cg.generate(10, SolveMode::NoAsymmetricCooperation, false);
    let mut produced_any = false;

    for clause in clauses {
        let (negated, positive): (Vec<i32>, Vec<i32>) =
            clause.iter().copied().partition(|&lit| lit < 0);
        if negated.len() != 1 || positive.len() != 1 {
            continue;
        }
        let Some(VarKey::Help {
            helper: _,
            beneficiary,
            t: help_t,
        }) = cg.engine.pool.key(positive[0])
        else {
            continue;
        };
        let Some(VarKey::Agent { agent_id, t, .. }) = cg.engine.pool.key(-negated[0]) else {
            continue;
        };
        assert_eq!(agent_id, beneficiary);
        assert_eq!(help_t, t);
        produced_any = true;
    }

    assert!(
        produced_any,
        "No beneficiary-to-help implication clauses were produced"
    );
}

/// `NoAsymmetricCooperation` must encode one-way help events into the global `Asymmetric`
/// variable, then forbid that variable by assumption.
#[test]
fn asymmetric_world_generates_forbid_clauses_and_assumptions() {
    let world = World::try_from(
        "
     S0 . S1
    L0E . .
     X  . X",
    )
    .expect("failed to parse world");
    let t_max = 10;
    let mut cg = ClauseGenerator::new(&world, t_max);
    let (clauses, assumptions) = cg.generate(10, SolveMode::NoAsymmetricCooperation, false);

    assert!(
        !clauses.is_empty(),
        "one-way world must produce asymmetric-definition clauses"
    );
    assert!(
        !assumptions.is_empty(),
        "one-way world must produce negative asymmetric assumptions"
    );
    for &lit in &assumptions {
        assert!(
            lit < 0,
            "all forbid-asymmetric assumptions must be negative literals"
        );
        assert!(
            matches!(
                cg.engine.pool.key(-lit),
                Some(VarKey::Asymmetric { horizon: 10 })
            ),
            "forbid-asymmetric assumption must negate the Asymmetric variable"
        );
    }
    assert!(
        clauses.iter().any(|clause| {
            clause.iter().any(|&lit| {
                lit > 0
                    && matches!(
                        cg.engine.pool.key(lit),
                        Some(VarKey::Asymmetric { horizon: 10 })
                    )
            })
        }),
        "definition clauses must imply the Asymmetric variable"
    );
}

/// The no-convergence mode materializes pair summaries, emits size-`k` blockers, and uses no assumptions.
#[test]
fn no_convergence_mode_generates_pairwise_blocking_clauses() {
    let world = World::try_from(
        "
         @  S0  S1  S2
        L0E  .   .   .
        L1E  .   .   .
         @   X   X   X
        ",
    )
    .expect("failed to parse convergence world");
    let horizon = 6;
    let mut generator = ClauseGenerator::new(&world, horizon);

    generator.generate(horizon, SolveMode::Standard, false);
    for helper in 0..world.n_agents() {
        for beneficiary in 0..world.n_agents() {
            assert!(!generator.exists(&VarKey::PairwiseHelp {
                helper,
                beneficiary,
                horizon,
            }));
        }
    }

    let (clauses, assumptions) =
        generator.generate(horizon, SolveMode::NoConvergentCooperation(2), false);
    assert!(assumptions.is_empty());

    let blockers = clauses
        .iter()
        .filter(|clause| {
            clause.len() == 2
                && clause.iter().all(|literal| {
                    *literal < 0
                        && matches!(
                            generator.engine.pool.key(-*literal),
                            Some(VarKey::PairwiseHelp {
                                horizon: pair_horizon,
                                ..
                            }) if pair_horizon == horizon
                        )
                })
        })
        .collect::<Vec<_>>();
    assert!(!blockers.is_empty());
    for blocker in blockers {
        let keys = blocker
            .iter()
            .map(|literal| generator.engine.pool.key(-*literal).unwrap())
            .collect::<Vec<_>>();
        let [
            VarKey::PairwiseHelp {
                helper: first_helper,
                beneficiary: first_beneficiary,
                ..
            },
            VarKey::PairwiseHelp {
                helper: second_helper,
                beneficiary: second_beneficiary,
                ..
            },
        ] = keys.as_slice()
        else {
            panic!("blocking clauses must contain only pairwise-help variables");
        };
        assert_ne!(first_helper, second_helper);
        assert_eq!(first_beneficiary, second_beneficiary);
    }
}

/// Reusing a generator keeps convergence summaries isolated across horizons and thresholds.
#[test]
fn no_convergence_generator_reuse_does_not_leak_horizons_or_thresholds() {
    let world = World::try_from(
        "
         @  S0  S1  S2
        L0E  .   .   .
        L1E  .   .   .
         @   X   X   X
        ",
    )
    .expect("failed to parse convergence world");
    let mut generator = ClauseGenerator::new(&world, 6);

    generator.generate(6, SolveMode::NoConvergentCooperation(2), false);
    let large_pairwise = (0..world.n_agents())
        .flat_map(|helper| {
            (0..world.n_agents()).filter_map(move |beneficiary| {
                (helper != beneficiary).then_some(VarKey::PairwiseHelp {
                    helper,
                    beneficiary,
                    horizon: 6,
                })
            })
        })
        .filter_map(|key| generator.literal(&key))
        .collect::<Vec<_>>();
    assert!(!large_pairwise.is_empty());

    let (small_clauses, _) = generator.generate(4, SolveMode::NoConvergentCooperation(2), false);
    assert!(large_pairwise.iter().all(|large| {
        !small_clauses
            .iter()
            .flatten()
            .any(|literal| literal.unsigned_abs() == *large as u32)
    }));
    assert!((0..world.n_agents()).any(|helper| {
        (0..world.n_agents()).any(|beneficiary| {
            generator.exists(&VarKey::PairwiseHelp {
                helper,
                beneficiary,
                horizon: 4,
            })
        })
    }));

    let variables_after_two = generator.n_vars();
    let (threshold_three, _) = generator.generate(4, SolveMode::NoConvergentCooperation(3), false);
    assert_eq!(generator.n_vars(), variables_after_two);
    assert!(!threshold_three.iter().any(|clause| {
        clause.len() == 3
            && clause.iter().all(|literal| {
                *literal < 0
                    && matches!(
                        generator.engine.pool.key(-*literal),
                        Some(VarKey::PairwiseHelp { horizon: 4, .. })
                    )
            })
    }));
}
