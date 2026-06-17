//! Direct tests of the `has_helped_by_time` / mutual-cooperation clause generation.
//!
//! These exercise the *structure* of the generated literals and clauses, independently of any
//! SAT solver (solving is delegated to Python). SAT/UNSAT behaviour is covered by the Python
//! tests in `python/tests/test_mutual_cooperation.py`.
//!
//! Note: a `has_helped_by_time` indicator is created only for `(helper, beneficiary)` pairs
//! that can take part in a *mutual* dependency, i.e. **both** agents own a laser. A non-owner can
//! never reciprocate help, so tracking a dependency onto it would be a dead variable. This is why
//! the single-owner worlds below produce no indicator at all.

use crate::solver::ClauseGenerator;
use crate::solver::SolveMode;
use crate::solver::VarKey;
use crate::World;

fn build(map: &str, t_max: usize, mode: SolveMode) -> ClauseGenerator {
    let world = World::try_from(map).expect("failed to parse world");
    let mut cg = ClauseGenerator::new(&world, t_max, mode);
    // `generate(t_max)` fills steps 0..=t_max in one call.
    let _ = cg.generate(t_max);
    cg
}

/// True if `helper` has a `has_helped_by_time(helper, beneficiary, t)` variable at any step.
fn can_help(cg: &ClauseGenerator, helper: usize, beneficiary: usize, t_max: usize) -> bool {
    (0..=t_max).any(|t| cg.exists(&VarKey::has_helped_by_time(helper, beneficiary, t)))
}


/// `S0` (laser owner) can step into beam `L0E` to protect `S1`, but `S1` owns no laser, so no
/// mutual dependency is even expressible.
const ONE_WAY: &str = "
 S0 . S1
L0E . .
 X  . X";

/// Two facing lasers, one per agent, each beam crossable by the *other* agent: mutual help is
/// geometrically possible in both directions.
const MUTUAL: &str = "
 S0 . . S1
L0E . . .
 .  . . L1W
 X  . . X";

/// No laser at all: nobody can ever help anyone.
const NO_LASER: &str = "
S0 . S1
 . . .
 X . X";

#[test]
fn single_owner_world_tracks_no_dependency() {
    // Only agent 0 owns a laser, so no mutual dependency is expressible: no indicator is created
    // in either direction and nothing is forbidden.
    let mut cg = build(ONE_WAY, 10, SolveMode::NoMutualCooperation);
    assert!(!can_help(&cg, 0, 1, 10), "no indicator onto a non-owner");
    assert!(!can_help(&cg, 1, 0, 10), "a non-owner can never help");
    let (clauses, assumptions) = cg.forbid_mutual_cooperation(10);
    assert!(
        clauses.is_empty() && assumptions.is_empty(),
        "a single-owner world cannot be mutual, so nothing is forbidden"
    );
}

#[test]
fn has_helped_by_time_clauses_are_binary_implications_into_has_helped() {
    let world = World::try_from(MUTUAL).expect("failed to parse world");
    let mut cg = ClauseGenerator::new(&world, 10, SolveMode::NoMutualCooperation);
    // Each clause must be a binary implication whose single positive literal is some
    // `has_helped_by_time(helper, beneficiary, t)`, and whose antecedent is either the
    // beneficiary's agent var (a fresh help event) or the previous-step indicator (monotone
    // carry-forward).
    let mut produced_any = false;
    for t in 0..=10 {
        for clause in cg.has_helped_by_time_clauses(t) {
            produced_any = true;
            assert_eq!(clause.len(), 2, "each implication must be a binary clause");
            let (negated, positive): (Vec<i32>, Vec<i32>) =
                clause.iter().copied().partition(|&l| l < 0);
            assert_eq!(negated.len(), 1, "exactly one negated (antecedent) literal");
            assert_eq!(
                positive.len(),
                1,
                "exactly one positive (has_helped) literal"
            );
            // The positive literal must be a has_helped_by_time indicator at the current step.
            let Some(VarKey::HasHelpedByTime {
                helper,
                beneficiary,
                t: has_helped_t,
            }) = cg.pool.key(positive[0])
            else {
                panic!("positive literal must be a HasHelpedByTime var");
            };
            assert_eq!(has_helped_t, t);
            // The antecedent is either the beneficiary's agent var, or the previous-step indicator.
            match cg.pool.key(-negated[0]) {
                Some(VarKey::Agent { agent_id, .. }) => assert_eq!(agent_id, beneficiary),
                Some(VarKey::HasHelpedByTime {
                    helper: h2,
                    beneficiary: b2,
                    t: prev_t,
                }) => {
                    assert_eq!((h2, b2), (helper, beneficiary));
                    assert_eq!(
                        prev_t,
                        t - 1,
                        "monotone carry must reference the previous step"
                    );
                }
                other => panic!("unexpected antecedent literal: {other:?}"),
            }
        }
    }
    assert!(
        produced_any,
        "two crossable facing beams must yield has-helped-by-time implications"
    );
}

#[test]
fn no_laser_has_no_dependencies() {
    let world = World::try_from(NO_LASER).expect("failed to parse world");
    let mut cg = ClauseGenerator::new(&world, 10, SolveMode::NoMutualCooperation);
    for t in 0..=10 {
        assert!(
            cg.has_helped_by_time_clauses(t).is_empty(),
            "no laser means no help events"
        );
    }
    let (clauses, assumptions) = cg.forbid_mutual_cooperation(10);
    assert!(clauses.is_empty() && assumptions.is_empty());
}

#[test]
fn mutual_world_creates_both_directions() {
    let cg = build(MUTUAL, 10, SolveMode::NoMutualCooperation);
    // agent 0 (L0E owner) can help agent 1 cross its east beam
    assert!(
        can_help(&cg, 0, 1, 10),
        "agent 0 should be able to help agent 1"
    );
    // agent 1 (L1W owner) can help agent 0 cross its west beam
    assert!(
        can_help(&cg, 1, 0, 10),
        "agent 1 should be able to help agent 0"
    );
}

#[test]
fn mutual_world_generates_forbid_clauses_and_assumptions() {
    let mut cg = build(MUTUAL, 10, SolveMode::NoMutualCooperation);
    let (clauses, assumptions) = cg.forbid_mutual_cooperation(10);
    assert!(
        !clauses.is_empty(),
        "mutual world must produce forbid clauses"
    );
    assert!(
        !assumptions.is_empty(),
        "mutual world must produce negative assumptions"
    );
    for &lit in &assumptions {
        assert!(
            lit < 0,
            "all forbid-mutual assumptions must be negative literals"
        );
    }
}

#[test]
fn asymmetric_world_generates_forbid_clauses_and_assumptions() {
    let mut cg = build(ONE_WAY, 10, SolveMode::NoAsymmetricCooperation);
    let (clauses, assumptions) = cg.forbid_asymmetric_cooperation(10);
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
            matches!(cg.pool.key(-lit), Some(VarKey::Asymmetric { .. })),
            "forbid-asymmetric assumption must negate an Asymmetric variable"
        );
    }
    assert!(
        clauses.iter().any(|clause| {
            clause
                .iter()
                .any(|&lit| lit > 0 && matches!(cg.pool.key(lit), Some(VarKey::Asymmetric { .. })))
        }),
        "definition clauses must imply an Asymmetric variable"
    );
}

#[test]
fn chained_mode_k2_enumerates_two_trails_for_mutual_world() {
    let world = World::try_from(MUTUAL).expect("failed to parse world");
    let cg = ClauseGenerator::new(&world, 10, SolveMode::NoChainedCooperation(2));
    // MUTUAL world: owners = [0, 1], all_agents = [0, 1].
    // Length-2 trails with no repeated directed pair: [0,1,0] and [1,0,1].
    assert_eq!(
        cg.walks.len(),
        2,
        "two distinct length-2 trails in a 2-owner world"
    );
}

#[test]
fn chained_mode_k3_has_no_trails_for_two_agent_world() {
    let world = World::try_from(MUTUAL).expect("failed to parse world");
    let cg = ClauseGenerator::new(&world, 10, SolveMode::NoChainedCooperation(3));
    // Only 2 distinct directed pairs (0->1) and (1->0) exist; a length-3 trail needs 3 distinct
    // directed pairs, which is impossible with 2 agents.
    assert!(
        cg.walks.is_empty(),
        "no length-3 trail can exist with only 2 agents and 2 possible directed pairs"
    );
}

#[test]
fn no_chained_mode_uses_walk_realized_variables() {
    let cg = build(MUTUAL, 10, SolveMode::NoChainedCooperation(2));
    // With trails [0,1,0] and [1,0,1], the MUTUAL world can realize both: walk_realized(0) and
    // walk_realized(1) should be allocated after generate().
    assert!(
        cg.exists(&VarKey::WalkRealized { walk_id: 0 }),
        "chain mode must allocate WalkRealized(0)"
    );
    assert!(
        cg.exists(&VarKey::WalkRealized { walk_id: 1 }),
        "chain mode must allocate WalkRealized(1)"
    );
}

#[test]
fn chained_mode_forbid_walks_produces_negative_assumptions() {
    let cg = build(MUTUAL, 10, SolveMode::NoChainedCooperation(2));
    let (clauses, assumptions) = cg.forbid_walks();
    assert!(clauses.is_empty(), "forbid_walks must not produce extra clauses");
    assert!(
        !assumptions.is_empty(),
        "mutual world with k=2 must have walk-realized assumptions"
    );
    for &lit in &assumptions {
        assert!(lit < 0, "all walk-forbid assumptions must be negative");
        assert!(
            matches!(cg.pool.key(-lit), Some(VarKey::WalkRealized { .. })),
            "assumption must negate a WalkRealized variable"
        );
    }
}

#[test]
fn level_6_dependency_is_bidirectional() {
    let world = World::get_level(6).expect("failed to load level 6");
    let n = world.n_agents();
    let mut cg = ClauseGenerator::new(&world, 21, SolveMode::NoMutualCooperation);
    let _ = cg.generate(21);
    // Level 6 requires mutual cooperation: at least one pair must have both directions.
    let has_bidirectional = (0..n).any(|a| {
        (0..n)
            .filter(|&b| b != a)
            .any(|b| can_help(&cg, a, b, 21) && can_help(&cg, b, a, 21))
    });
    assert!(
        has_bidirectional,
        "level 6 must have at least one bidirectional dependency pair"
    );
}
