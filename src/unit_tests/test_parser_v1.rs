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
fn test_box_next_to_a_void_is_fine() {
    // `V` and `B` cannot share a cell in v1, so `InvalidBoxPosition` is only
    // reachable through TOML — covered in Task 8.
    assert!(crate::World::try_from("S0 B X\nV  . .").is_ok());
}

#[test]
fn test_box_starting_on_a_laser_is_allowed() {
    // A box may start inside a beam: the beam is pre-blocked from reset.
    let world = crate::World::try_from("L0E B . X\n S0 . . .").unwrap();
    assert_eq!(world.n_boxes(), 1);
}

mod box_validation {
    use super::super::{LaserConfig, WorldConfig};
    use crate::{ParseError, Position, tiles::Direction};

    fn pos(i: usize, j: usize) -> Position {
        Position { i, j }
    }

    /// A 1x6 world with one agent, an exit at (0, 5) and the given occupants.
    fn config(
        boxes: Vec<Position>,
        starts: Vec<Vec<Position>>,
        voids: Vec<Position>,
        walls: Vec<Position>,
        sources: Vec<(Position, LaserConfig)>,
    ) -> WorldConfig {
        let n_agents = starts.len();
        WorldConfig::new(
            6,
            1,
            vec![],
            starts,
            voids,
            vec![pos(0, 5)],
            walls,
            boxes,
            sources,
            (0..n_agents).collect(),
        )
    }

    fn assert_invalid(result: Result<crate::World, ParseError>, position: Position) {
        match result.err() {
            Some(ParseError::InvalidBoxPosition { position: p }) => assert_eq!(p, position),
            other => panic!("expected InvalidBoxPosition, got {other:?}"),
        }
    }

    #[test]
    fn valid_box_is_accepted() {
        let c = config(
            vec![pos(0, 2)],
            vec![vec![pos(0, 0)]],
            vec![],
            vec![],
            vec![],
        );
        assert!(c.into_world().is_ok());
    }

    #[test]
    fn box_on_wall_is_rejected() {
        let c = config(
            vec![pos(0, 2)],
            vec![vec![pos(0, 0)]],
            vec![],
            vec![pos(0, 2)],
            vec![],
        );
        assert_invalid(c.into_world(), pos(0, 2));
    }

    #[test]
    fn box_on_void_is_rejected() {
        let c = config(
            vec![pos(0, 2)],
            vec![vec![pos(0, 0)]],
            vec![pos(0, 2)],
            vec![],
            vec![],
        );
        assert_invalid(c.into_world(), pos(0, 2));
    }

    #[test]
    fn box_on_laser_source_is_rejected() {
        let source = LaserConfig {
            direction: Direction::East,
            agent_id: 0,
            laser_id: 0,
        };
        let c = config(
            vec![pos(0, 2)],
            vec![vec![pos(0, 0)]],
            vec![],
            vec![],
            vec![(pos(0, 2), source)],
        );
        assert_invalid(c.into_world(), pos(0, 2));
    }

    #[test]
    fn box_on_agent_start_is_rejected() {
        let c = config(
            vec![pos(0, 0)],
            vec![vec![pos(0, 0)]],
            vec![],
            vec![],
            vec![],
        );
        assert_invalid(c.into_world(), pos(0, 0));
    }

    #[test]
    fn box_on_any_random_start_candidate_is_rejected() {
        let c = config(
            vec![pos(0, 2)],
            vec![vec![pos(0, 0), pos(0, 2)]],
            vec![],
            vec![],
            vec![],
        );
        assert_invalid(c.into_world(), pos(0, 2));
    }

    #[test]
    fn duplicate_boxes_are_rejected() {
        let c = config(
            vec![pos(0, 2), pos(0, 2)],
            vec![vec![pos(0, 0)]],
            vec![],
            vec![],
            vec![],
        );
        assert_invalid(c.into_world(), pos(0, 2));
    }
}
