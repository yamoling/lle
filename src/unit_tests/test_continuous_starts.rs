//! `ContinuousWorld::possible_starts` must be the starts validated by the shared map layer, i.e.
//! exactly the possible starts of the discrete `World` built from the same map.
//!
//! The pruning by a box blocking a beam has no parity case here: the continuous engine rejects
//! maps with boxes (`UnsupportedFeature`). The shared pruning itself, boxes included, is tested
//! with explicit expected starts in `test_map_spec.rs`.
use crate::continuous::{ContinuousError, ContinuousWorld, PhysicsParams};
use crate::{MapSpec, ValidMap, World};

/// Other-colour starts on the beam are pruned up to the wall; the start behind the wall is kept.
const OTHER_COLOUR: &str = r#"
world_string = """
L0E . . . @ .
S0  . . . . X
.   . . . . X
"""
[[agents]]
[[agents]]
start_positions = [{i=0,j=1},{i=0,j=2},{i=0,j=5},{i=2,j=2}]
"#;

/// The single start of a same-colour agent on the beam blocks it: the cells behind are kept.
const SAME_COLOUR_BLOCKS: &str = r#"
world_string = """
L0E . S0 . .
.   . .  . X
.   . .  . X
"""
[[agents]]
[[agents]]
start_positions = [{i=0,j=1},{i=0,j=3},{i=2,j=3}]
"#;

/// Lasers are processed in order: `L1E` leaves agent 0 with the single start (2, 2), which then
/// blocks `L0E`, so agent 1 keeps (2, 3).
const SEQUENTIAL: &str = r#"
world_string = """
L1E . . . .
.   . . . .
L0E . . . X
.   . . . X
"""
[[agents]]
start_positions = [{i=0,j=2},{i=2,j=2}]
[[agents]]
start_positions = [{i=2,j=3},{i=3,j=3}]
"#;

/// The same blocking rule on a v1 map: `S0` blocks its own beam, so `S1` does not lose its only
/// start (which would make the map invalid).
const V1_SAME_COLOUR_BLOCKS: &str = "L0E S0 . S1\n.   .  X X";

fn valid_map(content: &str) -> ValidMap {
    MapSpec::parse(content).unwrap().validate().unwrap()
}

#[test]
fn possible_starts_match_the_discrete_world_on_builtin_levels() {
    for level in 1..=6 {
        let continuous = ContinuousWorld::get_level(level).unwrap();
        let discrete = World::get_level(level).unwrap();
        assert_eq!(
            continuous.possible_starts(),
            discrete.possible_starts().as_slice(),
            "level {level}"
        );
    }
}

#[test]
fn possible_starts_match_the_discrete_world_on_pruned_maps() {
    for content in [
        OTHER_COLOUR,
        SAME_COLOUR_BLOCKS,
        SEQUENTIAL,
        V1_SAME_COLOUR_BLOCKS,
    ] {
        let continuous = ContinuousWorld::try_from(content).unwrap();
        let discrete = World::from_map(&valid_map(content));
        assert_eq!(
            continuous.possible_starts(),
            discrete.possible_starts().as_slice(),
            "map:\n{content}"
        );
    }
}

#[test]
fn from_map_borrows_the_valid_map_and_uses_its_starts() {
    // `from_map` takes the map by reference: no clone, no re-validation on the caller's side.
    // That `from_map` neither clones nor re-validates the map internally is checked by inspection
    // of `ContinuousWorld::from_map`: this test cannot observe it.
    let from_map: fn(&ValidMap, PhysicsParams) -> Result<ContinuousWorld, ContinuousError> =
        ContinuousWorld::from_map;
    for content in [OTHER_COLOUR, SEQUENTIAL] {
        let valid = valid_map(content);
        let world = from_map(&valid, PhysicsParams::default()).unwrap();
        // The map is still usable after the construction, and its starts are the pruned ones.
        assert_eq!(world.possible_starts(), valid.starts().as_slice());
    }
}
