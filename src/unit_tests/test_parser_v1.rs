use crate::ParseError;

use super::parse;

#[test]
fn test_laser_kill_on_spawn() {
    let config = parse(
        "
    L1S  X  .
     S0 S1  X
    ",
    )
    .unwrap();
    let world = config.to_world();
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
    let world = config.to_world();
    match world {
        Ok(_) => {}
        Err(ParseError::AgentWithoutStart { .. }) => panic!(
            "The start location of agent 0 should have been removed and no remaining start position remains for agent 0"
        ),
        Err(ParseError::NotEnoughExitTiles { .. }) => panic!("There are enough exit tiles"),
        Err(e) => panic!("Unexpected error: {:?}", e),
    }
}
