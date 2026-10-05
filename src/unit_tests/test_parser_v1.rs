use crate::{ParseError, Position, tiles::Direction};

use super::parse;

#[test]
fn test_multi_digit_start_agent_id() {
    let config = parse("S10 X").unwrap();

    // `S10` declares a single agent of colour 10 (see agent-colour-id.md §3.4b): a gap in the
    // token numbering is a sparse colour space, not ten missing agents.
    assert_eq!(config.n_agents(), 1);
    assert_eq!(config.colours(), &vec![10]);
    assert_eq!(config.random_starts()[0], vec![Position { i: 0, j: 0 }]);
}

#[test]
fn test_multi_digit_laser_source_agent_id() {
    let config = parse("L10E S0 S1 S2 S3 S4 S5 S6 S7 S8 S9 S10 X").unwrap();
    let (_, source) = &config.sources()[0];

    assert_eq!(source.agent_id, 10);
    assert_eq!(source.direction, Direction::East);
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
        assert_eq!(source.direction, Direction::West);
        assert_eq!(
            *source_pos,
            Position {
                i: agent_id + 1,
                j: 1
            }
        );
    }

    config.into_world().unwrap();
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
    assert_eq!(config.boxes(), &vec![Position { i: 0, j: 1 }]);
}

#[test]
fn test_several_boxes_keep_reading_order() {
    let config = parse("S0 B X\n.  B .").unwrap();
    assert_eq!(
        config.boxes(),
        &vec![Position { i: 0, j: 1 }, Position { i: 1, j: 1 }]
    );
}

#[test]
fn test_lowercase_box_token() {
    let config = parse("S0 b X").unwrap();
    assert_eq!(config.boxes(), &vec![Position { i: 0, j: 1 }]);
}

#[test]
fn test_world_without_box_has_no_boxes() {
    assert!(parse("S0 . X").unwrap().boxes().is_empty());
}

#[test]
fn test_box_is_emitted_in_the_v1_string() {
    let config = parse("S0 B . X").unwrap();
    assert_eq!(config.to_string(), "S0   B   .   X ");
}

#[test]
fn test_box_round_trips_through_v1() {
    let world = crate::World::try_from("S0 B . X").unwrap();
    let round_tripped = crate::World::try_from(world.world_string()).unwrap();
    assert_eq!(
        round_tripped.boxes_positions(),
        vec![Position { i: 0, j: 1 }]
    );
}

#[test]
fn test_box_starting_on_a_laser_is_allowed() {
    // A box may start inside a beam: the beam is pre-blocked from reset.
    let world = crate::World::try_from("L0E B . X\n S0 . . .").unwrap();
    assert_eq!(world.n_boxes(), 1);
}
