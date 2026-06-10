//! Direct tests of the dependency / mutual-cooperation clause generation
//! (`ClauseGenerator::dependency_clauses` and `forbid_mutual_cooperation`).
//!
//! These exercise the *structure* of the generated literals and clauses, independently of any
//! SAT solver (solving is delegated to Python). SAT/UNSAT behaviour is covered by the Python
//! tests in `python/tests/test_mutual_cooperation.py`.

use crate::World;
use crate::solver::ClauseGenerator;
use crate::solver::VarKey;

/// Build a generator and generate every world-consistency and dependency clause up to `t_max`,
/// returning the populated generator.
fn build(map: &str, t_max: usize) -> ClauseGenerator {
    let world = World::try_from(map).expect("failed to parse world");
    let mut cg = ClauseGenerator::new(&world, t_max);
    for t in 0..=t_max {
        let _ = cg.generate(t);
        let _ = cg.dependency_clauses(t);
    }
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
    let mut cg = ClauseGenerator::new(&world, 10);
    for t in 0..=10 {
        let _ = cg.generate(t);
    }
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
fn mutual_creates_both_directions_and_forbids_the_pair() {
    let mut cg = build(MUTUAL, 10);
    assert!(
        cg.exists(&VarKey::depends_on(1, 0)),
        "agent 0 can help agent 1"
    );
    assert!(
        cg.exists(&VarKey::depends_on(0, 1)),
        "agent 1 can help agent 0"
    );

    let d_ab = cg.pool.get(&VarKey::depends_on(1, 0)).unwrap(); // agent 0 helps agent 1
    let d_ba = cg.pool.get(&VarKey::depends_on(0, 1)).unwrap(); // agent 1 helps agent 0

    let (clauses, assumptions) = cg.forbid_mutual_cooperation();
    assert_eq!(clauses.len(), 1, "exactly one expressible pair");
    assert_eq!(assumptions.len(), 1);

    let mutual = cg.mutual_lit(0, 1).expect("mutual variable must now exist");
    // Definition clause: depends_on(1,0) ∧ depends_on(0,1) → mutual(0,1).
    let mut clause = clauses[0].clone();
    clause.sort();
    let mut expected = vec![-d_ab, -d_ba, mutual];
    expected.sort();
    assert_eq!(clause, expected);
    // The assumption asserts ¬mutual(0,1).
    assert_eq!(assumptions, vec![-mutual]);
}

#[test]
fn no_laser_has_no_dependencies() {
    let world = World::try_from(NO_LASER).expect("failed to parse world");
    let mut cg = ClauseGenerator::new(&world, 10);
    for t in 0..=10 {
        let _ = cg.generate(t);
        assert!(
            cg.dependency_clauses(t).is_empty(),
            "no laser means no help events"
        );
    }
    let (clauses, assumptions) = cg.forbid_mutual_cooperation();
    assert!(clauses.is_empty() && assumptions.is_empty());
}
