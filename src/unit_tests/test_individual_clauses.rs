//! Unit tests for the cooperation-tracking clause generators (`coop_clauses`,
//! `finalize_depends_on`, `depends_on_lit`). The non-cooperation generators are
//! exercised end-to-end through the Python solver test-suite.

use crate::World;
use crate::solver::ClauseGenerator;

/// The proof-of-concept level used by `lle.cooperation.characterize`: agent 0's
/// laser fires east along row 1, and agent 1 must be let through (short plans) or
/// detour around the wall (longer plans).
const POC_LEVEL: &str = "\
. . S0 S1 . .
L0E . . . @ .
. . . . . .
. . . . . .
X X . . . .";

fn poc_generator(t_max: usize) -> ClauseGenerator {
    let world = World::try_from(POC_LEVEL).expect("failed to parse PoC level");
    ClauseGenerator::new(&world, t_max)
}

#[test]
fn coop_clauses_create_depends_on_for_interacting_pair() {
    let mut cg = poc_generator(12);
    for t in 0..=12 {
        cg.generate(t);
        cg.coop_clauses(t);
    }
    cg.finalize_depends_on(12);
    // Agent 0's laser can protect agent 1, so depends_on(beneficiary=1, helper=0) exists.
    assert!(
        cg.depends_on_lit(1, 0).is_some(),
        "agent 1 can depend on agent 0 (its laser protects the beneficiary)"
    );
}

#[test]
fn depends_on_lit_is_none_before_finalize() {
    let mut cg = poc_generator(12);
    for t in 0..=12 {
        cg.generate(t);
        cg.coop_clauses(t);
    }
    // No finalize_depends_on call yet, so no depends_on variable can exist.
    assert!(cg.depends_on_lit(1, 0).is_none());
    assert!(cg.depends_on_lit(0, 1).is_none());
}

#[test]
fn depends_on_lit_is_none_for_self_pair() {
    let mut cg = poc_generator(12);
    for t in 0..=12 {
        cg.generate(t);
        cg.coop_clauses(t);
    }
    cg.finalize_depends_on(12);
    // An agent never "helps itself": no depends_on(a, a) is ever created.
    assert!(cg.depends_on_lit(0, 0).is_none());
    assert!(cg.depends_on_lit(1, 1).is_none());
}

#[test]
fn coop_clauses_are_empty_when_no_laser_can_reach_a_beneficiary() {
    // No lasers at all: there can be no cooperation, hence no coop clauses and no
    // depends_on variables for any horizon.
    let world = World::try_from("S0 . X\nS1 . X").expect("failed to parse laserless world");
    let mut cg = ClauseGenerator::new(&world, 6);
    for t in 0..=6 {
        cg.generate(t);
        assert!(
            cg.coop_clauses(t).is_empty(),
            "a laserless world produces no cooperation clauses"
        );
    }
    cg.finalize_depends_on(6);
    assert!(cg.depends_on_lit(0, 1).is_none());
    assert!(cg.depends_on_lit(1, 0).is_none());
}
