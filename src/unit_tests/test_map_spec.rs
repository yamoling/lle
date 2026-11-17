//! Tests of the pure map layer: `MapSpec::beam_cells`, `MapSpec::validate` (start pruning), the
//! single loader, and the parity of `World::from_map` with the former `WorldConfig::into_world`.
//!
//! The expected starts were captured on `origin/dev` (0414e02), before the map layer was made
//! pure.

use super::MapSpec;
use crate::{ParseError, Position, World, tiles::CardinalDirection};

fn pos(i: usize, j: usize) -> Position {
    Position::new2d(i, j)
}

fn spec(content: &str) -> MapSpec {
    MapSpec::parse(content).unwrap()
}

/// `(i, j)` of each start of each agent, to compare against compact literals.
fn compact(starts: &[Vec<Position>]) -> Vec<Vec<(usize, usize)>> {
    starts
        .iter()
        .map(|s| s.iter().map(|p| (p.i, p.j)).collect())
        .collect()
}

// ---------------------------------------------------------------------------
// beam_cells
// ---------------------------------------------------------------------------

#[test]
fn beam_stops_at_wall() {
    let map = spec("L0E . . @ . X\nS0 . . . . X");
    let beam = map.beam_cells(pos(0, 0), CardinalDirection::East);
    assert_eq!(beam, vec![pos(0, 1), pos(0, 2)]);
}

#[test]
fn beam_stops_at_another_source() {
    let map = spec("L0E . . L1W . X\nS0 . . S1 . X");
    assert_eq!(
        map.beam_cells(pos(0, 0), CardinalDirection::East),
        vec![pos(0, 1), pos(0, 2)]
    );
    assert_eq!(
        map.beam_cells(pos(0, 3), CardinalDirection::West),
        vec![pos(0, 2), pos(0, 1)]
    );
}

#[test]
fn beam_stops_at_grid_edge() {
    let map = spec(". L0S .\nS0 . .\n. # X\n. V .");
    // Boxes, voids and starts do not stop a beam: only walls, sources and the edge do.
    assert_eq!(
        map.beam_cells(pos(0, 1), CardinalDirection::South),
        vec![pos(1, 1), pos(2, 1), pos(3, 1)]
    );
    // A source on the edge, pointing outwards, has an empty beam.
    assert!(
        map.beam_cells(pos(0, 1), CardinalDirection::North)
            .is_empty()
    );
}

#[test]
fn beam_of_a_source_outside_the_layers_is_empty() {
    let map = spec("L0E . X\nS0 . .");
    let outside = Position { i: 0, j: 0, k: 1 };
    assert!(map.beam_cells(outside, CardinalDirection::East).is_empty());
}

// ---------------------------------------------------------------------------
// validate: purity and pruning parity
// ---------------------------------------------------------------------------

/// Other-colour starts on the beam are pruned, up to the wall; the cell behind it is kept.
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

/// A box on the beam blocks it: the starts behind the box are kept.
const BOX_BLOCKS: &str = r#"
world_string = """
L0E . # . .
S0  . . . X
.   . . . X
"""
[[agents]]
[[agents]]
start_positions = [{i=0,j=1},{i=0,j=3},{i=2,j=3}]
"#;

/// The single start of a same-colour agent on the beam blocks it.
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

/// The lasers are processed in order: `L1E` prunes (0, 2) from agent 0, which is left with the
/// single start (2, 2). That start then blocks `L0E`, so agent 1 keeps (2, 3).
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

/// The starts obtained on `origin/dev` before the refactoring, for each fixture.
#[allow(clippy::type_complexity)]
fn expected_fixtures() -> Vec<(&'static str, Vec<Vec<(usize, usize)>>)> {
    vec![
        (OTHER_COLOUR, vec![vec![(1, 0)], vec![(0, 5), (2, 2)]]),
        (BOX_BLOCKS, vec![vec![(1, 0)], vec![(0, 3), (2, 3)]]),
        (SAME_COLOUR_BLOCKS, vec![vec![(0, 2)], vec![(0, 3), (2, 3)]]),
        (SEQUENTIAL, vec![vec![(2, 2)], vec![(2, 3), (3, 3)]]),
    ]
}

#[test]
fn validate_prunes_like_before() {
    for (content, expected) in expected_fixtures() {
        let valid = spec(content).validate().unwrap();
        assert_eq!(compact(valid.starts()), expected, "map:\n{content}");
        let world = World::from_map(&valid);
        assert_eq!(
            compact(&world.possible_starts()),
            expected,
            "map:\n{content}"
        );
    }
}

#[test]
fn validate_does_not_mutate_the_spec() {
    for (content, _) in expected_fixtures() {
        let parsed = spec(content).random_starts().clone();
        let valid = spec(content).validate().unwrap();
        assert_eq!(valid.spec().random_starts(), &parsed);
        assert_ne!(valid.starts(), &parsed, "every fixture prunes a start");
    }
}

#[test]
fn validate_rejects_an_agent_whose_starts_are_all_pruned() {
    let map = spec(
        r#"
world_string = """
L0E . .
S0  . X
.   . X
"""
[[agents]]
[[agents]]
start_positions = [{i=0,j=1},{i=0,j=2}]
"#,
    );
    assert!(matches!(
        map.validate(),
        Err(ParseError::AgentWithoutStart { agent_id: 1 })
    ));
}

#[test]
fn validate_rejects_a_box_on_a_start() {
    let map = spec(
        r#"
world_string = "S0 # X"
[[agents]]
start_positions = [{i=0,j=1}]
"#,
    );
    assert!(matches!(
        map.validate(),
        Err(ParseError::InvalidBoxPosition { .. })
    ));
}

// ---------------------------------------------------------------------------
// Loader
// ---------------------------------------------------------------------------

#[test]
fn level_zero_is_an_error() {
    assert!(matches!(
        MapSpec::level(0),
        Err(ParseError::InvalidLevel {
            asked: 0,
            min: 1,
            max: 6
        })
    ));
    assert!(matches!(
        World::get_level(0),
        Err(ParseError::InvalidLevel { asked: 0, .. })
    ));
    assert!(matches!(
        World::get_level(7),
        Err(ParseError::InvalidLevel { asked: 7, .. })
    ));
}

#[test]
fn invalid_level_names_fall_back_to_files() {
    for name in ["lvl0", "level0", "lvl7", "lvlabc"] {
        assert!(
            matches!(
                MapSpec::from_file(name),
                Err(ParseError::InvalidFileName { .. })
            ),
            "{name}"
        );
    }
    assert!(MapSpec::from_file("LeVeL6").is_ok());
}

#[test]
fn level_starts_are_unchanged() {
    let expected: [Vec<Vec<(usize, usize)>>; 6] = [
        vec![vec![(0, 7)]],
        vec![vec![(0, 7)], vec![(0, 6)]],
        vec![vec![(0, 7)], vec![(0, 8)]],
        vec![vec![(0, 7)], vec![(0, 8)]],
        vec![vec![(0, 4)], vec![(0, 5)], vec![(0, 6)], vec![(0, 7)]],
        vec![vec![(0, 4)], vec![(0, 5)], vec![(0, 6)], vec![(0, 7)]],
    ];
    for (n, expected) in (1..=6).zip(expected) {
        let world = World::get_level(n).unwrap();
        assert_eq!(compact(&world.possible_starts()), expected, "level {n}");
        assert!(world.possible_starts().iter().flatten().all(|p| p.k == 0));
        let from_file = World::from_file(&format!("lvl{n}")).unwrap();
        assert_eq!(from_file.possible_starts(), world.possible_starts());
    }
}

#[test]
fn config_round_trip_keeps_starts_and_lasers() {
    for (content, _) in expected_fixtures() {
        let world = World::try_from(content).unwrap();
        let clone = world.clone();
        assert_eq!(clone.possible_starts(), world.possible_starts());
        let lasers =
            |w: &World| -> Vec<Position> { w.lasers().into_iter().map(|(p, _)| p).collect() };
        assert_eq!(lasers(&clone), lasers(&world));
    }
}
