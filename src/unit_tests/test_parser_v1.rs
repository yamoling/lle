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
