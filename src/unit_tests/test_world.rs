use core::panic;

use crate::{
    agent::Agent, reward, tiles::Laser, world::WorldState, Action, ParseError, RuntimeWorldError,
    World,
};

fn get_laser(world: &World, pos: (usize, usize)) -> &Laser {
    for (p, laser) in &world.lasers {
        if pos == *p {
            return laser;
        }
    }
    panic!("No laser at position {:?}", pos);
}

fn get_lasers(world: &World, pos: (usize, usize)) -> Vec<&Laser> {
    let mut lasers = vec![];
    for (p, laser) in &world.lasers {
        if pos == *p {
            lasers.push(laser.as_ref());
        }
    }
    lasers
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
    assert!(world.start_positions.contains(&(0, 0)));
    //let start = world.start_positions[0].get(&(0, 0)).unwrap();
    assert_eq!(world.start_positions[0], (0, 0));

    assert!(world
        .gems
        .iter()
        .map(|(pos, _)| pos)
        .any(|pos| pos.0 == 0 && pos.1 == 2));
    let source = &world
        .sources
        .iter()
        .find(|((i, j), _)| *i == 1 && *j == 0)
        .unwrap()
        .1;
    assert_eq!(source.agent_id(), 0);
    let laser = get_laser(&world, (1, 1));
    assert_eq!(laser.agent_id(), 0);
    let n_exits_at_1_1 = world
        .exits
        .iter()
        .filter(|(pos, _)| pos.0 == 1 && pos.1 == 1)
        .count();
    assert!(n_exits_at_1_1 == 1);
    assert!(world.wall_positions.len() == 2);
    assert!(world.wall_positions.contains(&(1, 2)));
    assert!(world.wall_positions.contains(&(1, 0)));
}

#[test]
fn test_duplicate_start_pos() {
    match World::try_from("S0 S0 X X") {
        Ok(..) => panic!("Should not be able to build a world with duplicate start positions"),
        Err(err) => match err {
            ParseError::DuplicateStartTile { start1, start2, .. } => {
                assert_eq!(start1, (0, 0));
                assert_eq!(start2, (0, 1));
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
    assert_eq!(world.agent_positions, vec![(0, 1), (0, 0)]);
    assert_eq!(world.start_positions, vec![(0, 1), (0, 0)]);
}

#[test]
fn test_start_pos_order_lvl6() {
    let mut world = World::from_file("lvl6").unwrap();
    assert_eq!(world.start_positions.len(), 4);
    world.reset();
    for (id, pos) in world.starts() {
        assert_eq!(*pos, (0, id + 4));
        assert_eq!(world.agent_positions[id], (0, id + 4));
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
    for (pos, _) in w.lasers {
        assert_ne!(pos, (2, 1));
        assert_ne!(pos, (3, 1));
    }
}

#[test]
fn test_laser_blocked_on_reset() {
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
    assert!(get_laser(&w, (1, 2)).is_on());
    assert!(get_laser(&w, (2, 2)).is_off());
    assert!(get_laser(&w, (3, 2)).is_off());
}

#[test]
fn test_facing_lasers() {
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
    assert!(!w.done());
    assert!(w.agents().iter().all(Agent::is_alive));
    for i in 1..3 {
        let lasers = get_lasers(&w, (i, 2));
        for laser in lasers {
            assert!(laser.is_off());
        }
    }
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
    assert!(w.done());
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
fn test_force_state() {
    let mut w = World::try_from(
        "
        S0 . G
        X  . .
    ",
    )
    .unwrap();
    w.reset();
    let s = WorldState {
        agents_positions: [(1, 2)].into(),
        gems_collected: [true].into(),
    };
    w.force_state(&s).unwrap();
    assert_eq!(w.agents_positions()[0], (1, 2));
    let gem = &w.gems[0].1;
    assert!(gem.is_collected());
    assert!(!w.done());
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
    let s = WorldState {
        agents_positions: [(1, 0)].into(),
        gems_collected: [true].into(),
    };
    w.force_state(&s).unwrap();
    assert_eq!(w.agents_positions()[0], (1, 0));
    let gem = &w.gems[0].1;
    assert!(gem.is_collected());
    assert!(w.done());
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
    assert!(w.agents[1].is_dead());
    assert!(w.done());
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
    let s = WorldState {
        agents_positions: [(1, 2), (0, 0)].into(),
        gems_collected: [true].into(),
    };
    match w.force_state(&s) {
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
    let s = WorldState {
        agents_positions: [(1, 2)].into(),
        gems_collected: [true, false].into(),
    };
    match w.force_state(&s) {
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
/// In the following map, if agent 0 is in (0, 2) and agent 1 is in (0, 3), agent 0 is blocking the laser.
/// When agent 1 leaves the tile and goes to the right, the laser should NOT activate.
fn test_complex_laser_blocking() {
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

    let laser = get_laser(&w, (0, 3));
    assert!(laser.is_on());

    let state = WorldState {
        agents_positions: [(0, 2), (0, 3)].into(),
        gems_collected: [false; 5].into(),
    };
    w.force_state(&state).unwrap();
    let laser = get_laser(&w, (0, 3));
    assert!(laser.is_off());
    assert!(w.agents().iter().all(Agent::is_alive));

    w.step(&[Action::Stay, Action::East]).unwrap();
    assert!(w.agents().iter().all(Agent::is_alive));
    let laser = get_laser(&w, (0, 3));
    assert!(laser.is_off());
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
    let s = WorldState {
        agents_positions: [(0, 0)].into(),
        gems_collected: [].into(),
    };
    w.force_state(&s).unwrap();
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
    assert!(w.done());
}

#[test]
fn test_reward_less_agents_than_exits() {
    let mut w = World::try_from(
        "
        S0 .
        X  X",
    )
    .unwrap();
    w.reset();
    assert_eq!(w.n_agents(), 1);
    let r = w.step(&[Action::South]).unwrap();
    assert_eq!(
        r,
        reward::REWARD_AGENT_JUST_ARRIVED + reward::REWARD_END_GAME
    );
}

#[test]
fn test_reward_less_agents_than_exits2() {
    let mut w = World::try_from(
        "
        . . . . .
        . @ . . .
        . @ X @ X
       S0 . . . . 
        V V V V V",
    )
    .unwrap();
    assert_eq!(w.n_agents(), 1);
    w.reset();

    let state = WorldState {
        agents_positions: [(1, 2)].into(),
        gems_collected: [].into(),
    };
    w.force_state(&state).unwrap();
    let r = w.step(&[Action::South]).unwrap();
    assert_eq!(
        r,
        reward::REWARD_AGENT_JUST_ARRIVED + reward::REWARD_END_GAME
    );
}
