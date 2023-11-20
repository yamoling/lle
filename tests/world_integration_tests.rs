use lle::{Action, ParseError, RuntimeWorldError, World, WorldEvent, WorldState};

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
fn test_force_state_agents_have_exited() {
    let mut w = World::try_from(
        "
        S0 . G
        X  . .
    ",
    )
    .unwrap();
    w.reset();
    let s = WorldState {
        agents_positions: [(1, 0)].into(),
        gems_collected: [true].into(),
    };
    w.force_state(&s).unwrap();
    for agent in w.agents() {
        assert!(agent.has_arrived());
    }
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

    let s = WorldState {
        agents_positions: [(1, 0), (1, 1)].into(),
        gems_collected: [false; 1].into(),
    };
    w.force_state(&s).unwrap();
    assert!(w.agents()[0].has_arrived());
    // Agent 1 should ne have arrived (it died before arriving)
    assert!(!w.agents()[1].has_arrived());
    // Agent 1 should be dead
    assert!(w.agents()[1].is_dead());
}

#[test]
/// In this test, agent 1 is standing in a laser of his own colour.
/// When it moves North, it should die from laser 2.
/// It is expected that the laser 1 should be released when agent 1 dies.
fn test_dead_agent_does_not_block_the_laser() {
    let mut w = World::try_from(
        "
        S0 .   G  X
        .  .  L2W X
        .  S1  .  X 
        . L1N  .  S2",
    )
    .unwrap();
    w.reset();
    let mut n_agents_dead = 0;
    for event in w
        .step(&[Action::East, Action::North, Action::Stay])
        .unwrap()
    {
        match event {
            WorldEvent::AgentDied { agent_id } => {
                assert!(agent_id == 0 || agent_id == 1);
                n_agents_dead += 1;
            }
            other => panic!("Expected AgentDied, got {:?}", other),
        }
    }
    assert_eq!(n_agents_dead, 2);
    // Check that the laser 1 is not blocked anymore
    for (pos, l) in w.lasers() {
        if *pos == (0, 1) {
            assert!(l.is_on());
        }
    }
}
