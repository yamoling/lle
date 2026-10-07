use crate::{
    ParseError, Position,
    tiles::{CardinalDirection, Tile, VerticalDirection},
};

use super::parse;

#[test]
fn test_multi_digit_start_agent_id() {
    let config = parse("S10 X").unwrap();

    // `S10` declares a single agent of colour 10 (see agent-colour-id.md §3.4b): a gap in the
    // token numbering is a sparse colour space, not ten missing agents.
    assert_eq!(config.n_agents(), 1);
    assert_eq!(config.colours(), &vec![10]);
    assert_eq!(config.random_starts()[0], vec![Position::new2d(0, 0)]);
}

#[test]
fn test_multi_digit_laser_source_agent_id() {
    let config = parse("L10E S0 S1 S2 S3 S4 S5 S6 S7 S8 S9 S10 X").unwrap();
    let (_, source) = &config.sources()[0];

    assert_eq!(source.agent_id, 10);
    assert_eq!(source.direction, CardinalDirection::East);
}

#[test]
fn test_multi_digit_agents_and_laser_sources_world() {
    let config = parse(
        " .   .   . . . .
         S0  L0W  . . . X
         S1  L1W  . . . X
         S2  L2W  . . . X
         S3  L3W  . . . X
         S4  L4W  . . . X
         S5  L5W  . . . X
         S6  L6W  . . . X
         S7  L7W  . . . X
         S8  L8W  . . . X
         S9  L9W  . . . X
         S10 L10W . . . X
         S11 L11W . . . X
         S12 L12W . . . X
         S13 L13W . . . X",
    )
    .unwrap();

    assert_eq!(config.n_agents(), 14);
    assert_eq!(config.sources().len(), 14);
    for (agent_id, (source_pos, source)) in config.sources().iter().enumerate() {
        assert_eq!(source.agent_id, agent_id);
        assert_eq!(source.direction, CardinalDirection::West);
        assert_eq!(*source_pos, Position::new2d(agent_id + 1, 1));
    }

    config.into_world().unwrap();
}

#[test]
fn test_parse_lift_and_button() {
    let config = parse(
        "
        S0 .  TU0A1
        .  B0 .
        .  .  X
        ",
    )
    .unwrap();
    let world = config.into_world().unwrap();

    match world.at(&Position { i: 0, j: 2, k: 0 }) {
        Some(Tile::Lift(lift)) => {
            assert_eq!(lift.direction(), VerticalDirection::Up);
            assert_eq!(lift.group_id(), 0);
            assert_eq!(lift.authorized_colour(), Some(1));
        }
        other => panic!("Expected a Lift tile, got {:?}", other),
    }

    match world.at(&Position { i: 1, j: 1, k: 0 }) {
        Some(Tile::Button(button)) => {
            assert_eq!(button.group_id(), 0);
            assert_eq!(button.authorized_colour(), None);
        }
        other => panic!("Expected a Button tile, got {:?}", other),
    }
}

#[test]
fn test_parse_lift_button_invalid_group_id() {
    match parse(
        "
        S0 TUx
        .  X
        ",
    ) {
        Err(ParseError::InvalidGroupId { .. }) => {}
        other => panic!("Expected ParseError::InvalidGroupId, got {:?}", other),
    }
}

#[test]
fn test_lift_button_round_trip() {
    let config = parse(
        "
        S0 .  TU0A1
        .  B0 .
        .  .  X
        ",
    )
    .unwrap();
    let as_string = super::to_v1_string(&config).unwrap();
    let reparsed = parse(&as_string).unwrap();
    let world = reparsed.into_world().unwrap();

    match world.at(&Position { i: 0, j: 2, k: 0 }) {
        Some(Tile::Lift(lift)) => {
            assert_eq!(lift.direction(), VerticalDirection::Up);
            assert_eq!(lift.group_id(), 0);
            assert_eq!(lift.authorized_colour(), Some(1));
        }
        other => panic!("Expected a Lift tile, got {:?}", other),
    }
    match world.at(&Position { i: 1, j: 1, k: 0 }) {
        Some(Tile::Button(button)) => {
            assert_eq!(button.group_id(), 0);
        }
        other => panic!("Expected a Button tile, got {:?}", other),
    }
}

#[test]
fn test_empty_string_returns_empty_world_not_panic() {
    match parse("") {
        Err(ParseError::EmptyWorld) => {}
        other => panic!("Expected ParseError::EmptyWorld, got {:?}", other),
    }
}

#[test]
fn test_leading_semicolon_returns_parse_error() {
    match parse(";\nS0 X") {
        Err(ParseError::EmptyWorld) => {}
        other => panic!("Expected ParseError::EmptyWorld, got {:?}", other),
    }
}

#[test]
fn test_doubled_semicolon_returns_parse_error() {
    match parse("S0 X\n;\n;") {
        Err(ParseError::Inconsistent3Dimensions { .. }) => {}
        other => panic!(
            "Expected ParseError::Inconsistent3Dimensions, got {:?}",
            other
        ),
    }
}

#[test]
fn test_row_length_mismatch_returns_inconsistent_2d() {
    match parse("X S0 .\n. .") {
        Err(ParseError::Inconsistent2Dimensions {
            expected_n_cols,
            actual_n_cols,
            row,
            ..
        }) => {
            assert_eq!(expected_n_cols, 3);
            assert_eq!(actual_n_cols, 2);
            assert_eq!(row, 1);
        }
        other => panic!(
            "Expected ParseError::Inconsistent2Dimensions, got {:?}",
            other
        ),
    }
}

#[test]
fn test_laser_kill_on_spawn() {
    let config = parse(
        "
    L1S  X  .
     S0 S1  X
    ",
    )
    .unwrap();
    let world = config.into_world();
    match world {
        Ok(_) => panic!(
            "The start location of agent 0 should have been removed and no remaining start position remains for agent 0"
        ),
        Err(ParseError::AgentWithoutStart { .. }) => {}
        Err(ParseError::NotEnoughExitTiles { .. }) => {}
        Err(e) => panic!("Unexpected error: {:?}", e),
    }
}

#[test]
fn test_laser_blocked_on_spawn() {
    let config = parse(
        "
    L1E . S1 S0 X
    L0E .  .  . X
    ",
    )
    .unwrap();
    let world = config.into_world();
    match world {
        Ok(_) => {}
        Err(ParseError::AgentWithoutStart { .. }) => panic!(
            "The start location of agent 0 should have been removed and no remaining start position remains for agent 0"
        ),
        Err(ParseError::NotEnoughExitTiles { .. }) => panic!("There are enough exit tiles"),
        Err(e) => panic!("Unexpected error: {:?}", e),
    }
}

#[test]
fn test_parse_box_token() {
    let config = parse("S0 B . X").unwrap();
    assert_eq!(config.boxes(), &vec![Position::new2d(0, 1)]);
}

#[test]
fn test_several_boxes_keep_reading_order() {
    let config = parse("S0 B X\n.  B .").unwrap();
    assert_eq!(
        config.boxes(),
        &vec![Position::new2d(0, 1), Position::new2d(1, 1)]
    );
}

#[test]
fn test_lowercase_box_token() {
    let config = parse("S0 b X").unwrap();
    assert_eq!(config.boxes(), &vec![Position::new2d(0, 1)]);
}

#[test]
fn test_world_without_box_has_no_boxes() {
    assert!(parse("S0 . X").unwrap().boxes().is_empty());
}

#[test]
fn test_box_is_emitted_in_the_v1_string() {
    let config = parse("S0 B . X").unwrap();
    assert_eq!(config.to_string(), "S0 B . X \n");
}

#[test]
fn test_box_round_trips_through_v1() {
    let world = crate::World::try_from("S0 B . X").unwrap();
    let round_tripped = crate::World::try_from(world.world_string()).unwrap();
    assert_eq!(round_tripped.boxes_positions(), vec![Position::new2d(0, 1)]);
}

#[test]
fn test_box_starting_on_a_laser_is_allowed() {
    // A box may start inside a beam: the beam is pre-blocked from reset.
    let world = crate::World::try_from("L0E B . X\n S0 . . .").unwrap();
    assert_eq!(world.n_boxes(), 1);
}
