use core::panic;

use crate::{
    agent::Agent, core::WorldState, tiles::Laser, Action, ParseError, Position, RuntimeWorldError,
    WorldEvent,
};

use super::World;

fn get_laser(world: &World, pos: Position) -> &Laser {
    for (p, laser) in world.lasers() {
        if pos == p {
            return laser;
        }
    }
    panic!("No laser at position {:?}", pos);
}

#[test]
fn test_tile_type() {
    let mut world = World::try_from(
        "
    S0 . G
    L0E X @
    ",
    )
    .unwrap();
    world.reset();
    assert!(world.start_positions.contains(&(Position { i: 0, j: 0 })));
    //let start = world.start_positions[0].get(&(0, 0)).unwrap();
    assert_eq!(world.start_positions[0], Position { i: 0, j: 0 });

    assert!(world
        .gems_positions
        .iter()
        .any(|pos| pos.i == 0 && pos.j == 2));
    let source = world
        .sources()
        .iter()
        .find(|(Position { i, j }, _)| *i == 1 && *j == 0)
        .unwrap()
        .1;
    assert_eq!(source.agent_id(), 0);
    let laser = get_laser(&world, Position { i: 1, j: 1 });
    assert_eq!(laser.agent_id(), 0);
    let n_exits_at_1_1 = world
        .exits
        .iter()
        .filter(|Position { i, j }| *i == 1 && *j == 1)
        .count();
    assert!(n_exits_at_1_1 == 1);
    assert!(world.wall_positions.len() == 2);
    assert!(world.wall_positions.contains(&Position { i: 1, j: 2 }));
    assert!(world.wall_positions.contains(&Position { i: 1, j: 0 }));
}

#[test]
fn test_duplicate_start_pos() {
    match World::try_from("S0 S0 X X") {
        Ok(..) => panic!("Should not be able to build a world with duplicate start positions"),
        Err(err) => match err {
            ParseError::DuplicateStartTile { start1, start2, .. } => {
                assert_eq!(start1, Position { i: 0, j: 0 });
                assert_eq!(start2, Position { i: 0, j: 1 });
            }
            other => panic!("Expected DuplicateStartTile error, got {other:?}"),
        },
    }
}

#[test]
fn test_start_pos_order() {
    let mut world = World::try_from("S1 S0 X X").unwrap();
    assert_eq!(world.start_positions.len(), 2);
    assert_eq!(world.start_positions[1], (0, 0));
    assert_eq!(world.start_positions[0], (0, 1));
    world.reset();
    assert_eq!(world.agents_positions, vec![(0, 1), (0, 0)]);
    assert_eq!(world.start_positions, vec![(0, 1), (0, 0)]);
}

#[test]
fn test_start_pos_order_lvl6() {
    let mut world = World::from_file("lvl6").unwrap();
    assert_eq!(world.start_positions.len(), 4);
    world.reset();
    for (id, pos) in world.starts().into_iter().enumerate() {
        assert_eq!(pos, (0, id + 4));
        assert_eq!(world.agents_positions[id], (0, id + 4));
    }
}

#[test]
fn test_laser_blocked_by_wall() {
    let mut w = World::try_from(
        "
        . L0S .
        .  .  . 
        X  @  S0 
        .  .  .",
    )
    .unwrap();
    w.reset();
    // Make sure the wall blocks the laser
    for pos in w.lasers_positions {
        assert_ne!(pos, (2, 1));
        assert_ne!(pos, (3, 1));
    }
}

#[test]
fn test_laser_blocked_on_reset() -> Result<(), RuntimeWorldError> {
    let mut w = World::try_from(
        "
        @ @ L0S @  @
        @ .  .  .  @
        @ X  S0 .  @
        @ .  .  .  @
        @ @  @  @  @",
    )
    .unwrap();
    w.reset();
    // All agents should be alive
    assert!(w.agents().iter().all(|a| a.is_alive()));
    assert!(get_laser(&w, (1, 2).into()).is_on());
    assert!(get_laser(&w, (2, 2).into()).is_off());
    assert!(get_laser(&w, (3, 2).into()).is_off());
    Ok(())
}

#[test]
fn test_facing_lasers() -> Result<(), RuntimeWorldError> {
    let mut w = World::try_from(
        "
         @ @ L0S @  @
         @ X  .  S0 @
         @ .  .  .  @
         @ X  .  S1 @
         @ @ L1N  @ @",
    )
    .unwrap();
    w.reset();
    w.step(&[Action::West, Action::West]).unwrap();
    assert!(w.agents().iter().all(Agent::is_alive));
    for (_, laser) in w.lasers() {
        assert!(laser.is_off());
    }
    Ok(())
}

#[test]
fn test_event_exit_when_staying() {
    let mut w = World::try_from(
        "S0 X .
         S1 . X",
    )
    .unwrap();
    w.reset();
    // The first time the agent exits, an event should be generated
    assert_eq!(1, w.step(&[Action::East, Action::Stay]).unwrap().len());
    // The second time, no event should be generated
    assert_eq!(0, w.step(&[Action::Stay, Action::Stay]).unwrap().len());
}

#[test]
fn test_facing_lasers_agent_dies() {
    let mut w = World::try_from(
        "
         @ @ L0S @  @
         @ X  .  S0 @
         @ .  .  .  @
         @ X  .  S1 @
         @ @ L1N  @ @",
    )
    .unwrap();
    w.reset();
    w.step(&[Action::West, Action::Stay]).unwrap();
    assert!(w.agents[0].is_dead());
}

#[test]
fn test_empty_world() {
    if let Err(e) = World::try_from("") {
        match e {
            ParseError::EmptyWorld => return,
            other => panic!("Wrong error type: {:?}", other),
        }
    }
    panic!("Should not be able to build a world from an empty string")
}

#[test]
fn test_force_state_invalid_number_of_agents() {
    let mut w = World::try_from(
        "
        S0 . G
        X  . .
    ",
    )
    .unwrap();
    w.reset();
    let s = WorldState::new_alive(
        [Position { i: 1, j: 2 }, Position { i: 0, j: 0 }].into(),
        [true].into(),
    );
    match w.set_state(&s) {
        Err(e) => match e {
            RuntimeWorldError::InvalidNumberOfAgents {
                given: actual,
                expected,
            } => {
                assert_eq!(actual, 2);
                assert_eq!(expected, 1);
            }
            other => panic!("Wrong error type: {:?}", other),
        },
        Ok(_) => panic!("Should not be able to force an invalid state"),
    }
}

#[test]
fn test_force_state_invalid_number_of_gems() {
    let mut w = World::try_from(
        "
        S0 . G
        X  . .
    ",
    )
    .unwrap();
    w.reset();
    let s = WorldState::new_alive([(1, 2).into()].into(), [true, false].into());
    match w.set_state(&s) {
        Err(e) => match e {
            RuntimeWorldError::InvalidNumberOfGems {
                given: actual,
                expected,
            } => {
                assert_eq!(actual, 2);
                assert_eq!(expected, 1);
            }
            other => panic!("Wrong error type: {:?}", other),
        },
        Ok(_) => panic!("Should not be able to force an invalid state"),
    }
}

#[test]
/// In this map, if agent 0 is in (0, 2) and agent 1 is in (0, 3), agent 0 is blocking the laser.
/// When agent 1 leaves the tile and goes to the right, the laser should NOT activate.
fn test_complex_laser_blocking() -> Result<(), RuntimeWorldError> {
    let mut w = World::try_from(
        "
    G L0E X . X
    G G . . L1W
    @ S0 . . @
    . @ . . .
    S1 G . . G",
    )
    .unwrap();
    w.reset();

    let laser = get_laser(&w, (0, 3).into());
    assert!(laser.is_on());

    let state = WorldState::new_alive([(0, 2).into(), (0, 3).into()].into(), [false; 5].into());
    w.set_state(&state).unwrap();
    let laser = get_laser(&w, (0, 3).into());
    assert!(laser.is_off());
    assert!(w.agents().iter().all(Agent::is_alive));

    w.step(&[Action::Stay, Action::East]).unwrap();
    assert!(w.agents().iter().all(Agent::is_alive));
    let laser = get_laser(&w, (0, 3).into());
    assert!(laser.is_off());
    Ok(())
}

#[test]
fn test_clone_after_step() {
    let mut w = World::try_from(
        "
        S0 . G
        X  . .
    ",
    )
    .unwrap();
    w.reset();
    w.step(&[Action::East]).unwrap();
    w.step(&[Action::East]).unwrap();

    let w2 = w.clone();
    assert_eq!(w2.agents_positions(), w.agents_positions());
    assert_eq!(w2.n_gems_collected(), w.n_gems_collected());
}

#[test]
fn test_set_state_available_actions() {
    let mut w = World::try_from(
        "
        .  . . @ . . . @ . X
        .  @ . @ . @ . @ . @
        S0 @ . . . @ . . . @
    ",
    )
    .unwrap();
    w.reset();
    let s = WorldState::new_alive([(0, 0).into()].into(), [].into());
    w.set_state(&s).unwrap();
    let actions = w.available_actions();
    assert_eq!(actions.len(), 1);
    assert_eq!(actions[0].len(), 3);
    assert!(actions[0].contains(&Action::South));
    assert!(actions[0].contains(&Action::Stay));
    assert!(actions[0].contains(&Action::East));
}

#[test]
fn test_die_in_void() {
    let mut w = World::try_from("S0 V X").unwrap();
    w.reset();
    w.step(&[Action::East]).unwrap();
    assert!(w.agents[0].is_dead());
}

#[test]
fn test_num_gems_collected() {
    let mut world = World::try_from("S0 G X").unwrap();
    world.reset();
    assert_eq!(world.n_gems_collected(), 0);
    world.step(&[Action::East]).unwrap();
    assert_eq!(world.n_gems_collected(), 1);
    world.step(&[Action::Stay]).unwrap();
    assert_eq!(world.n_gems_collected(), 1);
    world.step(&[Action::East]).unwrap();
    assert_eq!(world.n_gems_collected(), 1);
}

#[test]
fn test_num_agents_arrived() {
    let mut world = World::try_from("S0 G X").unwrap();
    world.reset();
    assert_eq!(world.n_agents_arrived(), 0);
    world.step(&[Action::East]).unwrap();
    assert_eq!(world.n_agents_arrived(), 0);
    world.step(&[Action::Stay]).unwrap();
    assert_eq!(world.n_agents_arrived(), 0);
    world.step(&[Action::East]).unwrap();
    assert_eq!(world.n_agents_arrived(), 1);
}

#[test]
fn parse_inconsistent_row_lengths() {
    match World::try_from(
        "X S0 .
         . .",
    ) {
        Ok(_) => panic!("Should not be able to parse worlds with inconsistent row lengths"),
        Err(e) => match e {
            ParseError::InconsistentDimensions {
                actual_n_cols,
                expected_n_cols,
                row,
            } => {
                assert_eq!(actual_n_cols, 2);
                assert_eq!(expected_n_cols, 3);
                assert_eq!(row, 1);
            }
            _ => panic!("Expected InconsistentDimensions, got {e:?}"),
        },
    }
}

#[test]
fn parse_inconsistent_start_exit_tiles() {
    match World::try_from("S1 S0 X") {
        Ok(_) => panic!("Should not be able to parse worlds with #exit < #start"),
        Err(e) => match e {
            ParseError::NotEnoughExitTiles { n_exits, n_starts } => {
                assert_eq!(n_starts, 2);
                assert_eq!(n_exits, 1);
            }
            _ => panic!("Expected InconsistentNumberOfAgents, got {e:?}"),
        },
    }
}

#[test]
fn parse_no_agents() {
    match World::try_from(". . G") {
        Ok(_) => panic!("Should not be able to create worlds without agents"),
        Err(e) => match e {
            ParseError::NoAgents => {}
            _ => panic!("Expected NoAgents, got {e:?}"),
        },
    }
}

#[test]
fn test_vertex_conflict() {
    let mut w = World::try_from(
        "
    S0 X .
    .  . .
    S1 X .",
    )
    .unwrap();
    w.reset();
    w.step(&[Action::South, Action::North]).unwrap();
    let pos = w.agents_positions();
    assert_eq!(pos[0], (0, 0));
    assert_eq!(pos[1], (2, 0));
}

#[test]
fn test_reset() {
    let mut w = World::try_from("S0 G X").unwrap();
    for _ in 0..10 {
        w.reset();
        assert_eq!(w.agents_positions()[0], (0, 0));
        assert_eq!(
            w.step(&[Action::East]).unwrap(),
            vec![WorldEvent::GemCollected { agent_id: 0 }]
        );
        assert_eq!(
            w.step(&[Action::East]).unwrap(),
            vec![WorldEvent::AgentExit { agent_id: 0 }]
        );
    }
}

#[test]
fn test_standard_levels() {
    for level in 1..7 {
        let name = format!("level{}", level);
        World::from_file(&name).unwrap();
        let name = format!("lvl{}", level);
        World::from_file(&name).unwrap();
    }
}

#[test]
fn test_get_level() {
    for level in 1..7 {
        World::get_level(level).unwrap();
    }
}

#[test]
fn test_force_state() {
    let mut w = World::try_from(
        "
        S0 . G
        X  . .
    ",
    )
    .unwrap();
    w.reset();
    let s = WorldState::new_alive([(1, 2).into()].into(), [true].into());
    w.set_state(&s).unwrap();
    assert_eq!(w.agents_positions()[0], (1, 2));
    let gem = w.gems()[0];
    assert!(gem.is_collected());
}

#[test]
fn test_force_end_state() {
    let mut w = World::try_from(
        "
        S0 . G
        X  . .
    ",
    )
    .unwrap();
    w.reset();
    let s = WorldState::new_alive([(1, 0).into()].into(), [true].into());
    w.set_state(&s).unwrap();
    assert_eq!(w.agents_positions()[0], (1, 0));
    let gem = w.gems()[0];
    assert!(gem.is_collected());
}

#[test]
fn test_force_state_agent_dies() {
    let mut w = World::try_from(
        "
        S0 S1 G
        X  X L0W
    ",
    )
    .unwrap();
    w.reset();

    let s = WorldState::new_alive([(1, 0).into(), (1, 1).into()].into(), [false; 1].into());
    w.set_state(&s).unwrap();
    assert!(w.agents()[1].is_dead());
}

#[test]
fn test_no_exits() {
    let toml_content = r#"
world_string = """
. . S0 . S1 . . . . S2 
. . .  . .  . . . . S3 
. . .  . .  . . . . . 
. . .  . .  . . . . . 
"""
"#;
    match World::try_from(toml_content) {
        Err(ParseError::NotEnoughExitTiles { .. }) => {}
        Ok(..) => panic!("Should not be able to create a world without exits"),
        Err(other) => panic!("Expected NotEnoughExitTiles, got {other:?}"),
    }
}
