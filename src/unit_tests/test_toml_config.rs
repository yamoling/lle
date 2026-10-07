use crate::{ParseError, Position, World};

use super::parse;

#[test]
fn invalid_toml_field() {
    let toml_content = r#"
    world_string = "S0 X"
    invalid_field = 25
    "#;
    match parse(toml_content) {
        Err(ParseError::UnknownTomlKey { key, .. }) => {
            assert_eq!(key, "invalid_field");
        }
        other => panic!("Should return a ParseError::UnknownTomlKey instead of {other:?}"),
    }
}

#[test]
fn invalid_toml_subfield() {
    let toml_content = r#"
    world_string = "S0 X"
    [[agents]]
    invalid_subfield = 25
    "#;
    match parse(toml_content) {
        Err(ParseError::UnknownTomlKey { key, .. }) => {
            assert_eq!(key, "invalid_subfield");
        }
        other => panic!("Should return a ParseError::UnknownTomlKey instead of {other:?}"),
    }
}

#[test]
fn parse_toml_width_problem() {
    match parse(
        r#"
width = 10
world_string = "S0 X"
"#,
    ) {
        Err(ParseError::InconsistentWorldStringWidth {
            toml_width,
            world_str_width,
        }) => {
            assert_eq!(toml_width, 10);
            assert_eq!(world_str_width, 2);
        }
        _ => panic!("Should return a ParseError::InconsistentWorldStringWidth"),
    }
}

#[test]
fn parse_toml_height_problem() {
    match parse(
        r#"
height = 10
world_string = "S0 X"
"#,
    ) {
        Err(ParseError::InconsistentWorldStringHeight {
            toml_height,
            world_str_height,
        }) => {
            assert_eq!(toml_height, 10);
            assert_eq!(world_str_height, 1);
        }
        _ => panic!("Should return a ParseError::InconsistentWorldStringWidth"),
    }
}

#[test]
fn parse_start_pos_rows() {
    match parse(
        r#"
height = 10
width = 10
n_agents = 2
starts = [{row = 0}]
"#,
    ) {
        Ok(config) => {
            // Start positions should be (0, 0), (0, 1) ... (0, 9)
            for start in config.random_starts() {
                for j in 0..config.width() {
                    assert!(start.contains(&Position { i: 0, j, k: 0 }));
                }
            }
        }
        _ => panic!("The TOML should be parsed successfully"),
    }
}

#[test]
fn parse_start_pos_cols() {
    match parse(
        r#"
height = 10
width = 10
n_agents = 2
starts = [{col = 0}]
"#,
    ) {
        Ok(config) => {
            for start in config.random_starts() {
                for i in 0..config.height() {
                    assert!(start.contains(&Position { i, j: 0, k: 0 }));
                }
            }
        }
        _ => panic!("The TOML should be parsed successfully"),
    }
}

#[test]
fn parse_start_pos_rows_and_cols() {
    match parse(
        r#"
height = 10
width = 10
n_agents = 2
starts = [{col = 0}, {row=0}]
"#,
    ) {
        Ok(config) => {
            for start in config.random_starts() {
                assert_eq!(
                    start.len(),
                    19,
                    "There should only be 19 starts since duplicates should be removed"
                );
                for i in 0..config.height() {
                    assert!(start.contains(&Position { i, j: 0, k: 0 }));
                }
                for j in 0..config.width() {
                    assert!(start.contains(&Position { i: 0, j, k: 0 }));
                }
            }
        }
        _ => panic!("The TOML should be parsed successfully"),
    }
}

#[test]
fn start_position_in_wall() {
    let toml_content = r#"
world_string="""
@ . .
@ . X
"""
[[agents]]
start_positions = [{i_min=1}]
    "#;
    let world = World::try_from(toml_content).unwrap();
    let starts = world.possible_starts();
    assert_eq!(1, starts[0].len())
}

#[test]
fn test_ok() {
    let toml_content = r#"
width = 10
height = 5
exits = [{ j_min = 9 }]
gems = [{ i = 0, j = 2 }]
world_string = """
X . . . S1 . . . . .
. . . . .  . . . . .
. . . . .  . . . . .
. . . . .  . . . . .
. . . . .  . . . . .
"""

[[agents]]
# Deduced from the string map: the `S1` token declares agent 0, of colour 1, at (0, 4).
# The token number is the agent's *colour*, not an index into this list (agent-colour-id.md
# §3.4): agents are created from the world string in `(colour, reading order)` order, then
# aligned with these blocks positionally.

[[agents]]
start_positions = [{ i_min = 0, i_max = 0 }]

[[agents]]
start_positions = [{ i = 0, j = 5 }, { i = 3, j = 5 }]

[[agents]]
start_positions = [
    { i = 4, j = 9 },
    { i_min = 1, i_max = 3, j_min = 0, j_max = 3 },
    { j_min = 4 },
]
"#;
    let w = World::try_from(toml_content).unwrap();
    assert_eq!(w.exits_positions().len(), 6);
    assert_eq!(w.gems_positions().len(), 1);
    assert_eq!(w.n_agents(), 4);
    // Agent 0 gets its only start from the world string, and its colour from the token.
    assert_eq!(w.possible_starts()[0], vec![Position { i: 0, j: 4, k: 0 }]);
    assert_eq!(w.agent_colours(), vec![1, 1, 2, 3]);
}

#[test]
fn test_global_start_pos() {
    let toml_content = r#"
width = 10
height = 10
n_agents = 5
starts = [{ row = 0 }]
exits = [{ col = 4 }]
"#;
    let w = World::try_from(toml_content).unwrap();
    assert_eq!(w.exits_positions().len(), 10);
    for starts in w.possible_starts() {
        assert_eq!(starts.len(), 9);
    }
}

// ---------------------------------------------------------------------------
// Movable boxes: `[[boxes]]`
// ---------------------------------------------------------------------------

fn pos(i: usize, j: usize) -> Position {
    Position::new2d(i, j)
}

/// Parses `toml` and returns the box positions, or the parse error.
fn parse_boxes(toml: &str) -> Result<Vec<Position>, ParseError> {
    World::try_from(toml).map(|w| w.boxes_positions())
}

#[test]
fn test_toml_boxes() {
    let toml = r#"
world_string = """
S0 . . X
.  . . .
"""
[[boxes]]
i = 1
j = 1
"#;
    let world = World::try_from(toml).unwrap();
    assert_eq!(world.n_boxes(), 1);
    assert_eq!(world.boxes_positions(), vec![pos(1, 1)]);
}

#[test]
fn test_toml_boxes_combine_with_the_world_string() {
    let toml = r#"
world_string = """
S0 B . X
.  . . .
"""
[[boxes]]
i = 1
j = 1
"#;
    let world = World::try_from(toml).unwrap();
    assert_eq!(world.n_boxes(), 2);
    let mut boxes = world.boxes_positions();
    boxes.sort_by_key(|p| (p.i, p.j));
    assert_eq!(boxes, vec![pos(0, 1), pos(1, 1)]);
}

#[test]
fn test_toml_boxes_range_expansion() {
    let toml = r#"
world_string = """
S0 . . X
.  . . .
"""
[[boxes]]
row = 1
"#;
    assert_eq!(parse_boxes(toml).unwrap().len(), 4);
}

/// `B` and `V` cannot share a cell in the v1 format, so TOML is the only route
/// to an illegal box position. This is the test for `InvalidBoxPosition`.
#[test]
fn test_toml_box_on_a_void_is_rejected() {
    let toml = r#"
world_string = """
S0 . V X
.  . . .
"""
[[boxes]]
i = 0
j = 2
"#;
    match parse_boxes(toml) {
        Err(ParseError::InvalidBoxPosition { position }) => assert_eq!(position, pos(0, 2)),
        other => panic!("expected InvalidBoxPosition, got {other:?}"),
    }
}

#[test]
fn test_toml_box_on_a_wall_is_rejected() {
    let toml = r#"
world_string = """
S0 . @ X
.  . . .
"""
[[boxes]]
i = 0
j = 2
"#;
    assert!(matches!(
        parse_boxes(toml),
        Err(ParseError::InvalidBoxPosition { .. })
    ));
}

#[test]
fn test_toml_box_on_a_laser_source_is_rejected() {
    let toml = r#"
world_string = """
S0 . L0E X
.  . .   .
"""
[[boxes]]
i = 0
j = 2
"#;
    match parse_boxes(toml) {
        Err(ParseError::InvalidBoxPosition { position }) => assert_eq!(position, pos(0, 2)),
        other => panic!("expected InvalidBoxPosition, got {other:?}"),
    }
}

#[test]
fn test_toml_box_on_an_agent_start_is_rejected() {
    let toml = r#"
world_string = """
S0 . . X
.  . . .
"""
[[boxes]]
i = 0
j = 0
"#;
    match parse_boxes(toml) {
        Err(ParseError::InvalidBoxPosition { position }) => assert_eq!(position, pos(0, 0)),
        other => panic!("expected InvalidBoxPosition, got {other:?}"),
    }
}

#[test]
fn test_toml_box_on_a_random_start_candidate_is_rejected() {
    let toml = r#"
world_string = """
. . . X
. . . .
"""
[[agents]]
start_positions = [{ i = 0, j = 0 }, { i = 1, j = 1 }]

[[boxes]]
i = 1
j = 1
"#;
    match parse_boxes(toml) {
        Err(ParseError::InvalidBoxPosition { position }) => assert_eq!(position, pos(1, 1)),
        other => panic!("expected InvalidBoxPosition, got {other:?}"),
    }
}

#[test]
fn test_toml_box_duplicated_with_the_world_string_is_rejected() {
    let toml = r#"
world_string = """
S0 B . X
.  . . .
"""
[[boxes]]
i = 0
j = 1
"#;
    match parse_boxes(toml) {
        Err(ParseError::InvalidBoxPosition { position }) => assert_eq!(position, pos(0, 1)),
        other => panic!("expected InvalidBoxPosition, got {other:?}"),
    }
}

#[test]
fn test_toml_box_duplicated_in_boxes_is_rejected() {
    let toml = r#"
world_string = """
S0 . . X
.  . . .
"""
[[boxes]]
i = 1
j = 1

[[boxes]]
i = 1
j = 1
"#;
    assert!(matches!(
        parse_boxes(toml),
        Err(ParseError::InvalidBoxPosition { .. })
    ));
}

#[test]
fn test_toml_box_out_of_bounds_is_rejected() {
    let toml = r#"
world_string = """
S0 . . X
"""
[[boxes]]
i = 5
j = 5
"#;
    assert!(parse_boxes(toml).is_err());
}

/// Plain TOML -> WorldConfig -> TOML -> WorldConfig keeps the boxes.
#[test]
fn test_toml_boxes_round_trip_through_toml_config() {
    use super::TomlConfig;
    let toml = r#"
world_string = """
S0 . . X
.  . . .
"""
[[boxes]]
i = 1
j = 1

[[boxes]]
i = 0
j = 2
"#;
    let config = parse(toml).unwrap();
    assert_eq!(config.boxes().len(), 2);
    let regenerated = TomlConfig::from(&config).to_toml_string();
    let reparsed = parse(&regenerated).unwrap();
    let mut boxes = reparsed.boxes().clone();
    boxes.sort_by_key(|p| (p.i, p.j));
    assert_eq!(boxes, vec![pos(0, 2), pos(1, 1)]);
}

/// Random starts make the v1 emission impossible, so `world_string()` falls back
/// to TOML. The boxes must survive that fallback.
#[test]
fn test_boxes_survive_the_toml_fallback_of_world_string_random_starts() {
    let toml = r#"
world_string = """
. . . X
. . . .
"""
[[agents]]
start_positions = [{ i = 0, j = 0 }, { i = 0, j = 1 }]

[[boxes]]
i = 1
j = 1
"#;
    let world = World::try_from(toml).unwrap();
    let string = world.world_string();
    assert!(
        string.contains("[[boxes]]"),
        "expected TOML fallback: {string}"
    );
    let reloaded = World::try_from(string.as_str()).unwrap();
    assert_eq!(reloaded.boxes_positions(), vec![pos(1, 1)]);
    assert_eq!(reloaded.possible_starts(), world.possible_starts());
}

/// Two agents of the same colour declared out of reading order also force the
/// TOML fallback (see `to_v1_string`).
#[test]
fn test_boxes_survive_the_toml_fallback_of_world_string_colour_order() {
    let toml = r#"
world_string = """
S0 . . X
S0 . . X
"""
[[agents]]
start_positions = [{ i = 1, j = 0 }]

[[agents]]
start_positions = [{ i = 0, j = 0 }]

[[boxes]]
i = 0
j = 2
"#;
    let world = World::try_from(toml).unwrap();
    let string = world.world_string();
    let reloaded = World::try_from(string.as_str()).unwrap();
    assert_eq!(reloaded.boxes_positions(), vec![pos(0, 2)]);
}

#[test]
fn test_toml_world_string_voids_survive() {
    let toml = r#"
world_string = """
S0 . V X
.  . . .
"""
[[boxes]]
i = 1
j = 1
"#;
    let world = World::try_from(toml).unwrap();
    assert_eq!(world.void_positions(), vec![pos(0, 2)]);
    assert_eq!(world.boxes_positions(), vec![pos(1, 1)]);
}

/// A box on a gem or an exit cannot be written in v1 (` B ` would erase the gem or exit), so
/// `world_string()` must fall back to TOML and keep both the box and what lies under it.
#[test]
fn test_world_string_round_trip_keeps_a_gem_under_a_box() {
    let toml = r#"
world_string = """
S0 G . X
"""
[[boxes]]
i = 0
j = 1
"#;
    let world = World::try_from(toml).unwrap();
    assert_eq!(world.boxes_positions(), vec![pos(0, 1)]);
    assert_eq!(world.gems_positions(), vec![pos(0, 1)]);
    let restored = World::try_from(world.world_string()).unwrap();
    assert_eq!(restored.gems_positions(), vec![pos(0, 1)]);
    assert_eq!(restored.boxes_positions(), vec![pos(0, 1)]);
    assert_eq!(restored.exits_positions(), world.exits_positions());
}

#[test]
fn test_world_string_round_trip_keeps_an_exit_under_a_box() {
    let toml = r#"
world_string = """
S0 X . X
"""
[[boxes]]
i = 0
j = 1
"#;
    let world = World::try_from(toml).unwrap();
    assert_eq!(world.boxes_positions(), vec![pos(0, 1)]);
    let restored = World::try_from(world.world_string()).unwrap();
    assert_eq!(restored.exits_positions(), world.exits_positions());
    assert_eq!(restored.exits_positions().len(), 2);
    assert_eq!(restored.boxes_positions(), vec![pos(0, 1)]);
}

#[test]
fn empty_world_string_returns_parse_error_not_panic() {
    match parse(r#"world_string = """#) {
        Err(ParseError::EmptyWorld) => {}
        other => panic!("Expected ParseError::EmptyWorld, got {other:?}"),
    }
}

#[test]
fn malformed_lift_table_returns_invalid_toml_document() {
    let toml_content = r#"
width = 3
height = 3
[[lifts]]
direction = "Up"
group_id = "oops"
[lifts.position]
i = 0
j = 0
k = 0
"#;
    match parse(toml_content) {
        Err(ParseError::InvalidTomlDocument { .. }) => {}
        other => panic!("Expected ParseError::InvalidTomlDocument, got {other:?}"),
    }
}

#[test]
fn malformed_button_table_missing_required_field() {
    let toml_content = r#"
width = 3
height = 3
[[buttons]]
[buttons.position]
i = 0
j = 0
k = 0
"#;
    match parse(toml_content) {
        Err(ParseError::InvalidTomlDocument { .. }) => {}
        other => panic!("Expected ParseError::InvalidTomlDocument, got {other:?}"),
    }
}

#[test]
fn unknown_field_in_lift_table_is_reported() {
    let toml_content = r#"
width = 3
height = 3
[[lifts]]
direction = "Up"
group_id = 0
directoin = "typo"
[lifts.position]
i = 0
j = 0
k = 0
"#;
    match parse(toml_content) {
        Err(ParseError::UnknownTomlKey { key, .. }) => {
            assert_eq!(key, "directoin");
        }
        other => panic!("Expected ParseError::UnknownTomlKey, got {other:?}"),
    }
}

#[test]
fn unknown_field_in_button_table_is_reported() {
    let toml_content = r#"
width = 3
height = 3
[[buttons]]
group_id = 0
gruop_id = 0
[buttons.position]
i = 0
j = 0
k = 0
"#;
    match parse(toml_content) {
        Err(ParseError::UnknownTomlKey { key, .. }) => {
            assert_eq!(key, "gruop_id");
        }
        other => panic!("Expected ParseError::UnknownTomlKey, got {other:?}"),
    }
}

#[test]
fn parse_toml_with_buttons_and_lifts_round_trip() {
    let toml_content = r#"
width = 3
height = 3
[[lifts]]
direction = "Up"
group_id = 1
authorized_colour = 0
[lifts.position]
i = 0
j = 2
k = 0

[[buttons]]
group_id = 1
[buttons.position]
i = 1
j = 1
k = 0
"#;
    let config = parse(toml_content).unwrap();
    assert_eq!(config.lifts().len(), 1);
    let (pos, lift) = &config.lifts()[0];
    assert_eq!(*pos, Position { i: 0, j: 2, k: 0 });
    assert_eq!(lift.direction, crate::tiles::VerticalDirection::Up);
    assert_eq!(lift.group_id, 1);
    assert_eq!(lift.authorized_colour, Some(0));

    assert_eq!(config.buttons().len(), 1);
    let (pos, button) = &config.buttons()[0];
    assert_eq!(*pos, Position { i: 1, j: 1, k: 0 });
    assert_eq!(button.group_id, 1);
    assert_eq!(button.authorized_colour, None);
}
