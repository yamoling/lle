use lle::{tiles::Laser, Action, ParseError, RuntimeWorldError, World, WorldEvent, WorldState};

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
    let s = WorldState::new_alive([(1, 0).into()].into(), [true].into());
    let events = w.set_state(&s).unwrap();
    for agent in w.agents() {
        assert!(agent.has_arrived());
    }
    assert_eq!(events.len(), 1);
    match &events[0] {
        WorldEvent::AgentExit { agent_id } => {
            assert_eq!(*agent_id, 0);
        }
        other => panic!("Expected AgentExit, got {:?}", other),
    }
}

#[test]
/// In this world state, agent 0 will block the laser.
/// Agent 1 enters a wall, which is not possible, so the world should raise an error.
/// We check that the laser is not blocked after the error and that the gem is not collected either.
fn test_force_wrong_state_check_laser_not_blocked() {
    let mut w = World::try_from(
        "
        S1  S0 X
        L0E  G  X
    ",
    )
    .unwrap();
    w.reset();
    let s = WorldState::new_alive([(1, 1).into(), (1, 0).into()].into(), [true].into());
    let res = w.set_state(&s);
    if let Err(RuntimeWorldError::InvalidAgentPosition { position, .. }) = res {
        assert_eq!(position, (1, 0));
    } else {
        panic!("Expected InvalidAgentPosition, got {:?}", res);
    }
    assert!(w.lasers().iter().all(|(_, l)| l.is_on()));
    assert!(w.gems().iter().all(|g| !g.is_collected()));
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
    assert!(w.agents()[0].has_arrived());
    // Agent 1 should ne have arrived (it died before arriving)
    assert!(!w.agents()[1].has_arrived());
    // Agent 1 should be dead
    assert!(w.agents()[1].is_dead());
}

#[test]
fn test_set_invalid_state() {
    let mut w = World::try_from(
        "
        S0 S1 X
        @  @  X
    ",
    )
    .unwrap();
    w.reset();
    match w.set_state(&WorldState::new_alive(
        vec![(1, 0).into(), (1, 1).into()],
        vec![],
    )) {
        Err(RuntimeWorldError::InvalidAgentPosition { .. }) => {}
        other => panic!("Expected InvalidState, got {:?}", other),
    }
    let pos = w.agents_positions();
    assert_eq!(pos[0], (0, 0));
    assert_eq!(pos[1], (0, 1));
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
        if pos == (0, 1) {
            assert!(l.is_on());
        }
    }
}

#[test]
fn world_state_equal() {
    let mut w = World::try_from(
        "
        S0 . G
        .  . X
    ",
    )
    .unwrap();
    w.reset();
    let s1 = w.get_state();
    let s2 = w.get_state();
    assert_eq!(s1, s2);
    w.step(&[Action::Stay]).unwrap();
    let s3 = w.get_state();
    assert_eq!(s1, s3);
    w.step(&[Action::East]).unwrap();
    let s4 = w.get_state();
    assert_ne!(s3, s4);
}

#[test]
fn change_laser_id() {
    let mut w = World::try_from(
        "
        S0 .   G  X
        .  .  L0W .
        .  S1  .  X 
        .  .   .  .",
    )
    .unwrap();
    w.reset();
    assert!(w.lasers().iter().all(|(_, l)| l.agent_id() == 0));
    {
        let (_, source) = w.sources()[0];
        source.set_agent_id(1);
        assert_eq!(source.agent_id(), 1);
    }
    assert!(w.lasers().iter().all(|(_, l)| l.agent_id() == 1));

    // Kill agent 0 in the laser
    let events = w.step(&[Action::South, Action::Stay]).unwrap();
    assert_eq!(events.len(), 1);
    match &events[0] {
        WorldEvent::AgentDied { agent_id } => {
            assert_eq!(*agent_id, 0);
        }
        other => panic!("Expected AgentDied, got {:?}", other),
    }
}

#[test]
fn disable_laser_source() {
    let mut w = World::try_from(
        "
        S0 .   G  X
        .  .  L0W .
        .  S1  .  X 
        .  .   .  .",
    )
    .unwrap();
    w.reset();
    assert!(w.lasers().iter().all(|(_, l)| l.is_on()));
    let (_, source) = w.sources()[0];
    source.disable();
    assert!(w.lasers().iter().all(|(_, l)| l.is_off()));
    source.enable();
    assert!(w.lasers().iter().all(|(_, l)| l.is_on()));
}

/// We check that disabling a laser source will turn off the lasers it is connected to,
/// even when an agent walks into the beam and then walks back away.
#[test]
fn disable_laser_source_and_block_with_agent() {
    let mut w = World::try_from("L0E . S0 X").unwrap();
    w.reset();

    fn get_laser_at(w: &World, pos: (usize, usize)) -> &Laser {
        w.lasers()
            .into_iter()
            .filter(|(p, _)| *p == pos)
            .map(|(_, l)| l)
            .collect::<Vec<_>>()
            .first()
            .unwrap()
    }
    let laser = get_laser_at(&w, (0, 1));
    assert!(laser.is_on());
    w.sources()[0].1.disable();
    assert!(laser.is_off());
    w.step(&[Action::West]).unwrap();
    let laser = get_laser_at(&w, (0, 2));
    assert!(laser.is_off());
    w.step(&[Action::East]).unwrap();
    let laser = get_laser_at(&w, (0, 1));
    assert!(laser.is_off());
}

#[test]
fn test_laser_id() {
    let mut w = World::try_from(
        "
        S0 .   G  X
        .  .  L0W .
        .  S1  .  X 
        .  .  L0W  .",
    )
    .unwrap();
    w.reset();
    let mut top_laser_id = None;
    let mut bot_laser_id = None;
    for (pos, source) in w.sources() {
        if pos.i == 1 {
            top_laser_id = Some(source.laser_id());
        } else if pos.i == 3 {
            bot_laser_id = Some(source.laser_id());
        } else {
            panic!("Unexpected laser source at ({}, {})", pos.i, pos.j);
        }
    }

    let top_laser_id = top_laser_id.unwrap();
    let bot_laser_id = bot_laser_id.unwrap();

    for (pos, l) in w.lasers() {
        if pos.i == 1 {
            assert_eq!(l.laser_id(), top_laser_id);
        } else if pos.i == 3 {
            assert_eq!(l.laser_id(), bot_laser_id);
        } else {
            panic!("Unexpected laser at ({}, {})", pos.i, pos.j);
        }
    }
}

#[test]
fn test_disable_laser_then_reset_does_not_turn_on() {
    let mut w = World::try_from("L0E . S0 X").unwrap();
    w.reset();
    w.sources()[0].1.disable();
    w.reset();
    let laser = w.lasers().iter().find(|(pos, _)| *pos == (0, 1)).unwrap().1;
    assert!(!laser.is_enabled());
    assert!(laser.is_disabled());
    assert!(laser.is_off());
}

#[test]
fn test_laser_sources_have_different_laser_ids() {
    let mut w = World::try_from("L0E . L0E . X S0").unwrap();
    w.reset();
    let laser_ids = w
        .sources()
        .iter()
        .map(|(_, l)| l.laser_id())
        .collect::<Vec<_>>();
    assert_eq!(laser_ids.len(), 2);
    assert_ne!(laser_ids[0], laser_ids[1]);
}

#[test]
fn test_wrong_agent_id_for_laser_source() {
    match World::try_from("S0 L5S X") {
        Ok(_) => panic!("Should not be able to parse world where a laser has an ID of an agent that does not exist."),
        Err(e) => match e {
            ParseError::InvalidLaserSourceAgentId { asked_id, n_agents } => {
                assert_eq!(asked_id, 5);
                assert_eq!(n_agents, 1);
            },
            other => panic!("Expected InvalidLaserSourceAgentId, got {:?}", other),
        }
    }
}

#[test]
fn test_compute_world_string() {
    let world = World::try_from("S0 L0S X").unwrap();
    let initial_string = world.initial_world_string().trim();
    let current_string = world.compute_world_string();
    let current_string = current_string.trim();
    assert_eq!(initial_string, current_string);

    let (_, source) = &world.sources()[0];
    source.set_agent_id(1);
    let expected = "S0 L1S X";
    let res = world.compute_world_string();
    let res = res.trim();
    assert_eq!(expected, res);
}

#[test]
fn test_laser_on_exit() {
    let mut w = World::try_from(
        "
    .   L0S S1
    S0   .   .
    L1E  X   X",
    )
    .unwrap();
    w.reset();
    let lasers = w.lasers();
    assert_eq!(lasers.len(), 4);
    for laser_id in 0..2 {
        let l: Vec<_> = lasers
            .iter()
            .filter(|(_, l)| l.laser_id() == laser_id)
            .collect();
        assert_eq!(l.len(), 2);
    }
}

#[test]
fn available_joint_actions() {
    let mut w = World::try_from(
        "S0 . S1 @
         @   X .  X",
    )
    .unwrap();
    w.reset();
    let available = w.available_joint_actions();
    assert_eq!(available.len(), 6);
    let expected = [
        [Action::Stay, Action::Stay],
        [Action::Stay, Action::West],
        [Action::Stay, Action::South],
        [Action::East, Action::Stay],
        [Action::East, Action::West],
        [Action::East, Action::South],
    ];
    for a in expected {
        assert!(available.contains(&a.to_vec()));
    }
}

#[test]
fn num_available_joint_actions() {
    let mut w = World::try_from(
        " X  S0  .  S1 @
         S2  @   X  .  X",
    )
    .unwrap();
    w.reset();
    let available = w.available_joint_actions();
    assert_eq!(available.len(), 2 * 3 * 3);
}

#[test]
fn test_world_state_dead_agents() {
    let mut w = World::try_from(
        "
    S0 . G
    V  . X
    ",
    )
    .unwrap();
    w.reset();
    let inital = w.get_state();
    assert!(inital.agents_alive[0]);
    w.step(&[Action::South]).unwrap();
    let state = w.get_state();
    assert!(!state.agents_alive[0]);

    w.reset();
    w.set_state(&state).unwrap();
    assert!(!w.agents()[0].is_alive());
}
