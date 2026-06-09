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

// ---------------------------------------------------------------------------
// finalize_mutual / mutual_lit
// ---------------------------------------------------------------------------

#[test]
fn mutual_lit_is_none_before_finalize() {
    let mut cg = poc_generator(12);
    for t in 0..=12 {
        cg.generate(t);
        cg.coop_clauses(t);
    }
    cg.finalize_depends_on(12);
    // finalize_mutual has not been called yet — no mutual variable can exist.
    assert!(cg.mutual_lit(0, 1).is_none());
    assert!(cg.mutual_lit(1, 0).is_none());
}

#[test]
fn mutual_lit_created_when_both_dep_vars_exist() {
    let mut cg = poc_generator(12);
    for t in 0..=12 {
        cg.generate(t);
        cg.coop_clauses(t);
    }
    // After finalize_depends_on, both depends_on(1,0) and depends_on(0,1) may exist.
    cg.finalize_depends_on(12);
    cg.finalize_mutual(12);
    // In the PoC level agent 0's laser can shield agent 1 (dep 1←0 exists) but agent 1
    // has no laser and cannot shield agent 0, so dep 0←1 is never created.
    // Therefore mutual(0,1) should also be absent.
    assert!(
        cg.mutual_lit(0, 1).is_none(),
        "mutual requires both directions of help to be reachable"
    );
}

#[test]
fn mutual_lit_is_canonical() {
    // mutual(a, b) and mutual(b, a) must refer to the same variable.
    let mut cg = poc_generator(12);
    for t in 0..=12 {
        cg.generate(t);
        cg.coop_clauses(t);
    }
    cg.finalize_depends_on(12);
    cg.finalize_mutual(12);
    // Whether or not the variable exists, both orderings must agree.
    assert_eq!(cg.mutual_lit(0, 1), cg.mutual_lit(1, 0));
}

#[test]
fn mutual_lit_absent_in_laserless_world() {
    let world = World::try_from("S0 . X\nS1 . X").expect("failed to parse laserless world");
    let mut cg = ClauseGenerator::new(&world, 6);
    for t in 0..=6 {
        cg.generate(t);
        cg.coop_clauses(t);
    }
    cg.finalize_depends_on(6);
    cg.finalize_mutual(6);
    assert!(cg.mutual_lit(0, 1).is_none());
}

// ---------------------------------------------------------------------------
// chain_clauses / finalize_chain / chain_lit
// ---------------------------------------------------------------------------

#[test]
fn chain_lit_is_none_before_finalize() {
    let mut cg = poc_generator(12);
    for t in 0..=12 {
        cg.generate(t);
        cg.coop_clauses(t);
        cg.chain_clauses(t);
    }
    // finalize_chain not called yet.
    assert!(cg.chain_lit(0, 1, 0).is_none());
}

#[test]
fn chain_lit_absent_with_only_two_agents() {
    // The PoC level has 2 agents; a chain requires 3 distinct agents (a, b, c).
    let mut cg = poc_generator(12);
    for t in 0..=12 {
        cg.generate(t);
        cg.coop_clauses(t);
        cg.chain_clauses(t);
    }
    cg.finalize_chain(12);
    // No valid triple exists with only agents 0 and 1.
    assert!(cg.chain_lit(0, 1, 0).is_none());
    assert!(cg.chain_lit(1, 0, 1).is_none());
}

#[test]
fn chain_clauses_empty_in_laserless_world() {
    // Without lasers there are no coop_event variables, so chain_clauses should
    // produce no clauses (nothing to chain from).
    let world = World::try_from("S0 . X\nS1 . X").expect("failed to parse laserless world");
    let mut cg = ClauseGenerator::new(&world, 6);
    for t in 0..=6 {
        cg.generate(t);
        cg.coop_clauses(t);
        assert!(
            cg.chain_clauses(t).is_empty(),
            "a laserless world produces no chain clauses at t={t}"
        );
    }
    cg.finalize_chain(6);
    assert!(cg.chain_lit(0, 1, 0).is_none());
}

#[test]
fn chain_lit_absent_when_no_triple_exists() {
    // Single-agent world: there are no (a, b, c) triples at all.
    let world = World::try_from("S0 . X").expect("failed to parse single-agent world");
    let mut cg = ClauseGenerator::new(&world, 4);
    for t in 0..=4 {
        cg.generate(t);
        cg.coop_clauses(t);
        cg.chain_clauses(t);
    }
    cg.finalize_chain(4);
    assert!(cg.chain_lit(0, 0, 0).is_none());
}
