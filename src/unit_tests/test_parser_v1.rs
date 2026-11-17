use crate::{
    ParseError, Position, World,
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

    assert_eq!(source.colour, 10);
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
        assert_eq!(source.colour, agent_id);
        assert_eq!(source.direction, CardinalDirection::West);
        assert_eq!(*source_pos, Position::new2d(agent_id + 1, 1));
    }

    World::try_from(config).unwrap();
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
    let world = World::try_from(config).unwrap();

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
    let world = World::try_from(reparsed).unwrap();

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
    let world = World::try_from(config);
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
    let world = World::try_from(config);
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
    let config = parse("S0 # . X").unwrap();
    assert_eq!(config.boxes(), &vec![Position::new2d(0, 1)]);
}

#[test]
fn test_several_boxes_keep_reading_order() {
    let config = parse("S0 # X\n.  # .").unwrap();
    assert_eq!(
        config.boxes(),
        &vec![Position::new2d(0, 1), Position::new2d(1, 1)]
    );
}

#[test]
fn test_world_without_box_has_no_boxes() {
    assert!(parse("S0 . X").unwrap().boxes().is_empty());
}

#[test]
fn test_box_is_emitted_in_the_v1_string() {
    let config = parse("S0 # . X").unwrap();
    assert_eq!(config.to_string(), "S0 # . X \n");
}

#[test]
fn test_box_round_trips_through_v1() {
    let world = crate::World::try_from("S0 # . X").unwrap();
    let round_tripped = crate::World::try_from(world.world_string()).unwrap();
    assert_eq!(round_tripped.boxes_positions(), vec![Position::new2d(0, 1)]);
}

#[test]
fn test_box_starting_on_a_laser_is_allowed() {
    // A box may start inside a beam: the beam is pre-blocked from reset.
    let world = crate::World::try_from("L0E # . X\n S0 . . .").unwrap();
    assert_eq!(world.n_boxes(), 1);
}

#[test]
fn test_parse_lever_and_pressure_plate() {
    let config = parse(
        "
        S0 H0  TU0
        .  P1A2 X
        ",
    )
    .unwrap();
    let world = World::try_from(config).unwrap();

    match world.at(&Position { i: 0, j: 1, k: 0 }) {
        Some(Tile::Lever(lever)) => {
            assert_eq!(lever.group_id(), 0);
            assert_eq!(lever.authorized_colour(), None);
            assert!(!lever.is_on());
        }
        other => panic!("Expected a Lever tile, got {:?}", other),
    }
    match world.at(&Position { i: 1, j: 1, k: 0 }) {
        Some(Tile::PressurePlate(plate)) => {
            assert_eq!(plate.group_id(), 1);
            assert_eq!(plate.authorized_colour(), Some(2));
        }
        other => panic!("Expected a PressurePlate tile, got {:?}", other),
    }
}

#[test]
fn test_parse_lever_and_pressure_plate_invalid_group_id() {
    for token in ["Hx", "Px", "H0Ax"] {
        let world_str = format!("S0 {token} X");
        assert!(
            matches!(
                parse(&world_str),
                Err(ParseError::InvalidGroupId { .. } | ParseError::InvalidAgentId { .. })
            ),
            "{token} should not parse"
        );
    }
}

#[test]
fn test_lever_and_pressure_plate_round_trip() {
    let config = parse("S0 H0A1 P2 TU0 X").unwrap();
    let as_string = super::to_v1_string(&config).unwrap();
    assert_eq!(
        as_string.split_whitespace().collect::<Vec<_>>(),
        ["S0", "H0A1", "P2", "TU0", "X"]
    );
}

/// The map is written like it is drawn: the first floor of the string is the top one, and the
/// last one is the bottom floor, `k = 0`.
#[test]
fn test_first_floor_of_the_string_is_the_top_floor() {
    let config = parse(
        "
        X  .
        ;
        .  .
        ;
        S0 TU0
        ",
    )
    .unwrap();
    assert_eq!(config.layers(), 3);
    assert_eq!(config.height(), 1);
    assert_eq!(config.width(), 2);
    assert_eq!(
        config.random_starts()[0],
        vec![Position { i: 0, j: 0, k: 0 }]
    );
    assert_eq!(config.exits(), &vec![Position { i: 0, j: 0, k: 2 }]);
    let (lift_pos, lift) = &config.lifts()[0];
    assert_eq!(*lift_pos, Position { i: 0, j: 1, k: 0 });
    assert_eq!(lift.direction, VerticalDirection::Up);
}

/// The tiles are listed from the bottom floor to the top one, each floor in reading order.
#[test]
fn test_tiles_are_listed_from_the_bottom_floor() {
    let config = parse(
        "
        G  .  X
        ;
        .  G  G
        ;
        S0 .  G
        ",
    )
    .unwrap();
    assert_eq!(
        config.gems(),
        &vec![
            Position { i: 0, j: 2, k: 0 },
            Position { i: 0, j: 1, k: 1 },
            Position { i: 0, j: 2, k: 1 },
            Position { i: 0, j: 0, k: 2 },
        ]
    );
}

/// Agents of the same colour get their ids from the bottom floor up.
#[test]
fn test_same_colour_agents_are_numbered_from_the_bottom_floor() {
    let config = parse("S0 X\n;\nS0 X").unwrap();
    assert_eq!(
        config.random_starts(),
        &vec![
            vec![Position { i: 0, j: 0, k: 0 }],
            vec![Position { i: 0, j: 0, k: 1 }],
        ]
    );
}

/// Writing a world back gives the floors in the order they were read, top floor first.
#[test]
fn test_multi_floor_round_trip_keeps_the_floor_order() {
    let map = "X . TD1\n;\nS0 TU0 TD1\n;\nS0 B0 X";
    let config = parse(map).unwrap();
    let as_string = super::to_v1_string(&config).unwrap();
    assert_eq!(
        as_string.split_whitespace().collect::<Vec<_>>(),
        map.split_whitespace().collect::<Vec<_>>()
    );
    let reparsed = parse(&as_string).unwrap();
    assert_eq!(reparsed.random_starts(), config.random_starts());
    assert_eq!(reparsed.exits(), config.exits());
}
