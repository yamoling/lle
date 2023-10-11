use lle::{
    Action, ParseError, RuntimeWorldError, World, WorldState, REWARD_AGENT_DIED,
    REWARD_AGENT_JUST_ARRIVED, REWARD_END_GAME, REWARD_GEM_COLLECTED,
};

#[test]
fn test_available_actions() {
    let mut w = World::try_from(
        "
    S0 . G
    L0E X .
    ",
    )
    .unwrap();
    w.reset();
    let available = w.available_actions();
    assert_eq!(available.len(), w.n_agents());
    assert_eq!(available[0].len(), 2);
    assert!(available[0].contains(&Action::Stay));
    assert!(available[0].contains(&Action::East));
}

#[test]
fn test_available_actions_two_agents() {
    let mut w = World::try_from(
        "
    .  S1 .
    .  S0 G
    L0E X X
    ",
    )
    .unwrap();
    w.reset();
    let available = w.available_actions();
    assert_eq!(available.len(), w.n_agents());
    let expected_0 = [Action::Stay, Action::East, Action::West, Action::South];
    let expected_1 = [Action::Stay, Action::East, Action::West];
    assert_eq!(available[0].len(), expected_0.len());
    assert_eq!(available[1].len(), expected_1.len());
    for a in &expected_0 {
        assert!(available[0].contains(a));
    }
    for a in &expected_1 {
        assert!(available[1].contains(a));
    }
}

#[test]
fn test_available_actions_exit() {
    let mut w = World::try_from(
        "
    .  S1 .
    .  S0 G
    L0E X X
    ",
    )
    .unwrap();
    w.reset();
    // On reset
    let available = w.available_actions();
    let expected_0 = [Action::Stay, Action::East, Action::West, Action::South];
    let expected_1 = [Action::Stay, Action::East, Action::West];
    assert_eq!(available[0].len(), expected_0.len());
    assert_eq!(available[1].len(), expected_1.len());
    assert!(expected_0.iter().all(|a| available[0].contains(a)));
    assert!(expected_1.iter().all(|a| available[1].contains(a)));

    // Step once (South, East)
    w.step(&[Action::South, Action::East]).unwrap();
    let available = w.available_actions();
    let expected_0 = [Action::Stay];
    let expected_1 = [Action::Stay, Action::West, Action::South];
    assert_eq!(available[0].len(), expected_0.len());
    assert_eq!(available[1].len(), expected_1.len());
    assert!(expected_0.iter().all(|a| available[0].contains(a)));
    assert!(expected_1.iter().all(|a| available[1].contains(a)));

    // Step once (Stay, South)
    w.step(&[Action::Stay, Action::South]).unwrap();
    let available = w.available_actions();
    let expected_0 = [Action::Stay];
    let expected_1 = [Action::Stay, Action::West, Action::South, Action::North];
    assert_eq!(available[0].len(), expected_0.len());
    assert_eq!(available[1].len(), expected_1.len());
    assert!(expected_0.iter().all(|a| available[0].contains(a)));
    assert!(expected_1.iter().all(|a| available[1].contains(a)));

    // Step once (Stay, South)
    w.step(&[Action::Stay, Action::South]).unwrap();
    let available = w.available_actions();
    let expected_0 = [Action::Stay];
    let expected_1 = [Action::Stay];
    assert_eq!(available[0].len(), expected_0.len());
    assert_eq!(available[1].len(), expected_1.len());
    assert!(expected_0.iter().all(|a| available[0].contains(a)));
    assert!(expected_1.iter().all(|a| available[1].contains(a)));
}

#[test]
fn parse_empty_world() {
    match World::try_from("") {
        Ok(_) => panic!("Should not be able to parse empty world"),
        Err(e) => match e {
            ParseError::EmptyWorld { .. } => {}
            _ => panic!("Expected EmptyWorld, got {e:?}"),
        },
    }
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
fn test_move_end_game() -> Result<(), RuntimeWorldError> {
    let mut w = World::try_from(
        "
    S0 X .
    .  . .
    .  . .",
    )
    .unwrap();
    w.reset();
    w.step(&[Action::South])?;
    assert!(!w.done());
    w.step(&[Action::South])?;
    assert!(!w.done());
    w.step(&[Action::East])?;
    assert!(!w.done());
    w.step(&[Action::North])?;
    assert!(!w.done());
    w.step(&[Action::North])?;
    assert!(w.done());
    Ok(())
}

#[test]
fn test_reward_finish() {
    let mut w = World::try_from(
        "
    S0 X .
    .  . .
    .  . .",
    )
    .unwrap();
    w.reset();
    assert_eq!(w.step(&[Action::East]).unwrap(), 2f32);
}

#[test]
fn test_reward_arrive_only_once() {
    let mut w = World::try_from(
        "
    S0 . G
    S1 X X",
    )
    .unwrap();
    w.reset();
    assert_eq!(w.step(&[Action::East, Action::East]).unwrap(), 1f32);
    assert_eq!(w.step(&[Action::East, Action::Stay]).unwrap(), 1f32);
    assert_eq!(w.step(&[Action::Stay, Action::Stay]).unwrap(), 0f32);
    assert_eq!(w.step(&[Action::South, Action::Stay]).unwrap(), 2f32);
}

/// After forcing the state of the environment, make sure that
/// the reward is correct, and does not include the reward of
/// forcing the state.
#[test]
fn test_reward_after_forced_state() {
    let mut w = World::try_from(
        "
    S0 . G
    S1 X X",
    )
    .unwrap();
    w.reset();
    let state = WorldState {
        agents_positions: vec![(0, 1), (1, 1)],
        gems_collected: vec![false],
    };
    w.force_state(&state).unwrap();
    assert_eq!(w.step(&[Action::East, Action::Stay]).unwrap(), 1f32);
}

#[test]
fn test_reward_force_state_all_arrived() {
    let mut w = World::try_from(
        "
    S0 . G
    S1 X X",
    )
    .unwrap();
    w.reset();
    let state = WorldState {
        agents_positions: vec![(0, 2), (1, 1)],
        gems_collected: vec![true],
    };

    w.force_state(&state).unwrap();
    assert_eq!(w.step(&[Action::South, Action::Stay]).unwrap(), 2f32);
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
fn test_reward() {
    let mut w = World::try_from(
        "
    S0 G .
    .  . X",
    )
    .unwrap();
    w.reset();
    assert_eq!(w.step(&[Action::East]).unwrap(), REWARD_GEM_COLLECTED);
    assert_eq!(w.step(&[Action::East]).unwrap(), 0f32);
    assert_eq!(
        w.step(&[Action::South]).unwrap(),
        REWARD_AGENT_JUST_ARRIVED + REWARD_END_GAME
    );
}

#[test]
fn test_reward_death() {
    let mut w = World::try_from(
        "
    S0 L0S X
    S1  .  X",
    )
    .unwrap();
    w.reset();

    assert_eq!(
        w.step(&[Action::Stay, Action::East]).unwrap(),
        REWARD_AGENT_DIED
    );
    assert!(w.done());
}

#[test]
fn test_reward_collect_and_death() {
    let mut w = World::try_from(
        "
    S0 L0S X
    S1  G  X",
    )
    .unwrap();
    w.reset();

    assert_eq!(
        w.step(&[Action::Stay, Action::East]).unwrap(),
        REWARD_AGENT_DIED
    );
    assert!(w.done());
}

#[test]
fn test_take_action_not_available() {
    let mut w = World::try_from("S0 X").unwrap();
    w.reset();
    if let Err(e) = w.step(&[Action::North]) {
        match e {
            RuntimeWorldError::InvalidAction {
                agent_id, taken, ..
            } => {
                assert_eq!(agent_id, 0);
                assert_eq!(taken, Action::North);
                return;
            }
            other => panic!("Expected InvalidAction, got {:?}", other),
        }
    }
    panic!("Expected the world to raise an error");
}

#[test]
fn test_take_action_not_available_swap() {
    let mut w = World::try_from("S0 X\nS1 X").unwrap();
    w.reset();
    if let Err(e) = w.step(&[Action::South, Action::North]) {
        match e {
            RuntimeWorldError::InvalidAction {
                agent_id, taken, ..
            } => {
                assert_eq!(agent_id, 0);
                assert_eq!(taken, Action::South);
                return;
            }
            other => panic!("Expected InvalidAction, got {:?}", other),
        }
    }
    panic!("Expected the world to raise an error");
}

#[test]
fn test_take_action_not_available_walk_into_laser_source() {
    let mut w = World::try_from("L0E X\nS0 .").unwrap();
    w.reset();
    if let Err(e) = w.step(&[Action::North]) {
        match e {
            RuntimeWorldError::InvalidAction {
                agent_id, taken, ..
            } => {
                assert_eq!(agent_id, 0);
                assert_eq!(taken, Action::North);
                return;
            }
            other => panic!("Expected InvalidAction, got {:?}", other),
        }
    }
    panic!("Expected the world to raise an error");
}

#[test]
fn test_take_action_walk_outside_map() {
    let mut w = World::try_from("L0E X\nS0 .").unwrap();
    w.reset();
    if let Err(e) = w.step(&[Action::West]) {
        match e {
            RuntimeWorldError::InvalidAction {
                agent_id, taken, ..
            } => {
                assert_eq!(agent_id, 0);
                assert_eq!(taken, Action::West);
                return;
            }
            other => panic!("Expected InvalidAction, got {:?}", other),
        }
    }
    panic!("Expected the world to raise an error");
}

#[test]
fn test_reset() {
    let mut w = World::try_from("S0 G X").unwrap();
    for i in 0..10 {
        w.reset();
        assert!(!w.done(), "World should not be done after reset (step {i})");
        assert_eq!(w.agents_positions()[0], (0, 0));
        assert_eq!(w.step(&[Action::East]).unwrap(), REWARD_GEM_COLLECTED);
        assert!(!w.done());
        assert_eq!(
            w.step(&[Action::East]).unwrap(),
            REWARD_AGENT_JUST_ARRIVED + REWARD_END_GAME
        );
        assert!(w.done());
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
