//! Tests for the `NoCooperation` assumption builder (`solver/clauses/assumptions.rs`).
//!
//! These assert the *structure* of the produced assumptions, independently of any SAT solver
//! (solving is delegated to Python).

use crate::World;
use crate::solver::VarKey;
use crate::solver::clauses::engine::ClauseEngine;

/// `assume_no_cooperation_until` must produce only negative literals that forbid the *non-owner*
/// from standing on a beam tile. In `ONE_WAY` only agent 1 is a non-owner, so every assumed-false
/// literal must reference agent 1, never the laser owner (agent 0).
#[test]
fn no_cooperation_assumptions_forbid_only_non_owners() {
    let world = World::try_from("L0S . X\n.   S1 X\nS0  . .").unwrap();
    let mut engine = ClauseEngine::new(&world, 2);
    let mut assumptions = Vec::new();
    for t in 0..=2 {
        assumptions.extend(engine.assume_no_cooperation_at(t));
    }
    for lit in assumptions {
        assert!(lit < 0, "every no-cooperation assumption must be negative");
        match engine.pool.key(lit.abs()) {
            Some(VarKey::Agent { agent_id, .. }) => assert_eq!(
                agent_id, 1,
                "only the non-owner (agent 1) may be forbidden from a beam tile"
            ),
            other => panic!("no-cooperation assumption must negate an Agent var, got {other:?}"),
        }
    }
}
