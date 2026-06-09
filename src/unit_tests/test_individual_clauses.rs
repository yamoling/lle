//! Unit tests for the cooperation-tracking clause generators (`coop_clauses`,
//! `finalize_depends_on`, `depends_on_lit`). The non-cooperation generators are
//! exercised end-to-end through the Python solver test-suite.

use crate::World;
use crate::solver::ClauseGenerator;

/// The proof-of-concept level used by `lle.cooperation.characterize`: agent 0's
/// laser fires east along row 1, and agent 1 must be let through (short plans) or
/// detour around the wall (longer plans).
fn poc_generator(t_max: usize) -> ClauseGenerator {
    let world = World::try_from(
        "
    . . S0 S1 . .
    L0E . . . @ .
    . . . . . .
    . . . . . .
    X X . . . .",
    )
    .expect("failed to parse PoC level");
    ClauseGenerator::new(&world, t_max)
}
