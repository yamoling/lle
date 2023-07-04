use crate::{
    agent::Agent,
    tiles::{Gem, Laser, LaserSource},
    Action, RuntimeWorldError, World, WorldError,
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
    assert!(world.starts.contains_key(&(0, 0)));
    let start = world.starts.get(&(0, 0)).unwrap();
    assert_eq!(start.agent_id(), 0);

    let _gem: &Gem = world.gems.get(&(0, 2)).unwrap();
    let source: &LaserSource = world.sources.get(&(1, 0)).unwrap();
    assert_eq!(source.agent_id(), 0);
    let laser = get_laser(&world, (1, 1));
    assert_eq!(laser.agent_id(), 0);
    let _exit = world.exits.get(&(1, 1)).unwrap();
    assert!(world.walls.contains(&(1, 2)));
    assert!(world.walls.len() == 1);
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
            WorldError::EmptyWorld => return,
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
    let agent_pos = [(1, 2)].into();
    let gem_collected = [true];
    w.force_state(agent_pos, &gem_collected).unwrap();
    assert_eq!(w.agent_positions()[0], (1, 2));
    let gem = w.gems.get(&(0, 2)).unwrap();
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
    let agent_pos = [(1, 0)].into();
    let gem_collected = [true];
    w.force_state(agent_pos, &gem_collected).unwrap();
    assert_eq!(w.agent_positions()[0], (1, 0));
    let gem = w.gems.get(&(0, 2)).unwrap();
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
    let agent_pos = [(0, 0), (1, 1)].into();
    let gem_collected = [true];
    w.force_state(agent_pos, &gem_collected).unwrap();
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
    let agent_pos = [(1, 2), (0, 0)].into();
    let gem_collected = [true];
    match w.force_state(agent_pos, &gem_collected) {
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
    let agent_pos = [(1, 2)].into();
    let gem_collected = [true, false];
    match w.force_state(agent_pos, &gem_collected) {
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

    w.force_state([(0, 2), (0, 3)].into(), &[false; 5]).unwrap();
    let laser = get_laser(&w, (0, 3));
    assert!(laser.is_off());
    assert!(w.agents().iter().all(Agent::is_alive));

    w.step(&[Action::Stay, Action::East]).unwrap();
    assert!(w.agents().iter().all(Agent::is_alive));
    let laser = get_laser(&w, (0, 3));
    assert!(laser.is_off());
}
