//! Direct tests of the dependency / mutual-cooperation clause generation.
//!
//! These exercise the *structure* of the generated literals and clauses, independently of any
//! SAT solver (solving is delegated to Python). SAT/UNSAT behaviour is covered by the Python
//! tests in `python/tests/test_mutual_cooperation.py`.

use crate::World;
use crate::solver::ClauseGenerator;
use crate::solver::SolveMode;
use crate::solver::VarKey;

/// Build a generator using `NoMutualCooperation` mode and fill all steps up to `t_max`.
fn build(map: &str, t_max: usize) -> ClauseGenerator {
    let world = World::try_from(map).expect("failed to parse world");
    let mut cg = ClauseGenerator::new(&world, t_max, SolveMode::NoMutualCooperation);
    // `generate(t_max)` fills steps 0..=t_max in one call.
    let _ = cg.generate(t_max);
    cg
}

/// `S0` (laser owner) can step into beam `L0E` to protect `S1`, but `S1` owns no laser, so the
/// dependency is strictly one-directional.
const ONE_WAY: &str = "
 S0 . S1
L0E . .
 X  . X";

/// Two facing lasers, one per agent, each beam crossable by the *other* agent: mutual help is
/// geometrically possible in both directions.
// const MUTUAL: &str = "
//  S0 . . S1
// L0E . . .
//  .  . . L1W
//  X  . . X";

/// No laser at all: nobody can ever help anyone.
const NO_LASER: &str = "
S0 . S1
 . . .
 X . X";

#[test]
fn one_way_creates_only_one_direction() {
    let cg = build(ONE_WAY, 10);
    // agent 0 (laser owner) can help agent 1 ...
    assert!(
        cg.exists(&VarKey::depends_on(1, 0)),
        "agent 0 should be able to help agent 1"
    );
    // ... but agent 1 owns no laser, so it can never help agent 0.
    assert!(
        !cg.exists(&VarKey::depends_on(0, 1)),
        "agent 1 owns no laser and cannot help agent 0"
    );
}

#[test]
fn dependency_clauses_are_binary_help_implications() {
    let world = World::try_from(ONE_WAY).expect("failed to parse world");
    let mut cg = ClauseGenerator::new(&world, 10, SolveMode::NoMutualCooperation);
    // Collect every dependency clause and check each is `agent(benef, q, t) → depends_on(1, 0)`.
    let mut produced_any = false;
    for t in 0..=10 {
        for clause in cg.dependency_clauses(t) {
            produced_any = true;
            assert_eq!(
                clause.len(),
                2,
                "each help implication must be a binary clause"
            );
            let (negated, positive): (Vec<i32>, Vec<i32>) =
                clause.iter().copied().partition(|&l| l < 0);
            assert_eq!(negated.len(), 1, "exactly one negated (agent) literal");
            assert_eq!(
                positive.len(),
                1,
                "exactly one positive (depends_on) literal"
            );
            // The negated literal is an agent variable for the *beneficiary* (agent 1).
            match cg.pool.key(-negated[0]) {
                Some(VarKey::Agent { agent_id, .. }) => assert_eq!(agent_id, 1),
                other => panic!("expected an agent literal, got {other:?}"),
            }
            // The positive literal is exactly depends_on(beneficiary=1, helper=0).
            assert_eq!(Some(positive[0]), cg.pool.get(&VarKey::depends_on(1, 0)));
        }
    }
    assert!(
        produced_any,
        "a crossable beam must yield help implications"
    );
}

#[test]
fn one_way_forbids_nothing() {
    let mut cg = build(ONE_WAY, 10);
    let (clauses, assumptions) = cg.forbid_mutual_cooperation();
    assert!(
        clauses.is_empty() && assumptions.is_empty(),
        "a one-directional dependency cannot be mutual, so nothing is forbidden"
    );
}

#[test]
fn no_laser_has_no_dependencies() {
    let world = World::try_from(NO_LASER).expect("failed to parse world");
    let mut cg = ClauseGenerator::new(&world, 10, SolveMode::NoMutualCooperation);
    for t in 0..=10 {
        assert!(
            cg.dependency_clauses(t).is_empty(),
            "no laser means no help events"
        );
    }
    let (clauses, assumptions) = cg.forbid_mutual_cooperation();
    assert!(clauses.is_empty() && assumptions.is_empty());
}
