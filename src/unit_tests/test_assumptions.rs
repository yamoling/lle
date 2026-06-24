//! Tests for the `NoCooperation` assumption builder (`solver/clauses/assumptions.rs`).
//!
//! These assert the *structure* of the produced assumptions, independently of any SAT solver
//! (solving is delegated to Python).

use crate::World;
use crate::solver::{ClauseGenerator, SolveMode, VarKey};

/// `S0` owns the single laser `L0E`; `S1` is a non-owner that can step onto the beam.
const ONE_WAY: &str = "
 S0 . S1
L0E . .
 X  . X";

fn build(map: &str, t_max: usize, mode: SolveMode) -> ClauseGenerator {
    let world = World::try_from(map).expect("failed to parse world");
    let mut cg = ClauseGenerator::new(&world, t_max);
    let _ = cg.generate(t_max, mode, false);
    cg
}

/// `assume_no_cooperation_until` must produce only negative literals that forbid the *non-owner*
/// from standing on a beam tile. In `ONE_WAY` only agent 1 is a non-owner, so every assumed-false
/// literal must reference agent 1, never the laser owner (agent 0).
#[test]
fn no_cooperation_assumptions_forbid_only_non_owners() {
    let mut cg = build(ONE_WAY, 10, SolveMode::NoCooperation);
    let assumptions = cg.assume_no_cooperation_until(10);
    assert!(
        !assumptions.is_empty(),
        "a crossable beam must yield no-cooperation assumptions"
    );
    for lit in assumptions {
        assert!(lit < 0, "every no-cooperation assumption must be negative");
        match cg.pool.key(lit.abs()) {
            Some(VarKey::Agent { agent_id, .. }) => assert_eq!(
                agent_id, 1,
                "only the non-owner (agent 1) may be forbidden from a beam tile"
            ),
            other => panic!("no-cooperation assumption must negate an Agent var, got {other:?}"),
        }
    }
}
