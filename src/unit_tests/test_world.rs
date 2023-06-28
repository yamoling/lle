use crate::{
    agent::Agent,
    tiles::{laser::Direction, TileType},
    Action, World,
};

#[test]
fn test_tile_type() {
    let w = World::try_from(
        "
    S0 . G
    L0E F .
    ",
    )
    .unwrap();
    match w.get((0, 0)).unwrap().tile_type() {
        TileType::Start { agent_num } => assert!(*agent_num == 0),
        other => panic!("Expected TileType Start, got {other:?}"),
    };
    match w.get((0, 1)).unwrap().tile_type() {
        TileType::Floor => {}
        other => panic!("Expected TileType Floor, got {other:?}"),
    };
    match w.get((0, 2)).unwrap().tile_type() {
        TileType::Gem { collected } => assert!(!collected),
        other => panic!("Expected TileType Gem, got {other:?}"),
    };
    match w.get((1, 0)).unwrap().tile_type() {
        TileType::LaserSource(source) => {
            assert!(source.agent_num() == 0);
            assert!(source.direction() == Direction::East);
        }
        other => panic!("Expected TileType Floor, got {other:?}"),
    };

    match w.get((1, 1)).unwrap().tile_type() {
        TileType::Laser(laser) => {
            assert!(laser.is_on());
            match laser.wrapped().tile_type() {
                TileType::Exit => {}
                other => panic!("Expected TileType Exit, got {other:?}"),
            }
        }
        other => panic!("Expected TileType Laser, got {other:?}"),
    };

    match w.get((1, 2)).unwrap().tile_type() {
        TileType::Laser(l) => {
            assert!(l.is_on());
            assert!(l.direction() == Direction::East);
            assert!(l.agent_num() == 0);
        }
        other => panic!("Expected TileType Laser, got {other:?}"),
    };
}

#[test]
fn test_laser_blocked_on_reset() {
    let mut w = World::try_from(
        "@ @ L0S @  @
             @ .  .  .  @
             @ F  S0 .  @
             @ .  .  .  @
             @ @  @  @  @",
    )
    .unwrap();
    w.reset();
    // All agents should be alive ()
    assert!(w.agents().iter().all(|a| a.is_alive()));
    let laser = w.get((3, 2)).unwrap();
    match laser.tile_type() {
        TileType::Laser(l) => {
            assert!(l.is_off());
        }
        other => panic!("Expected Laser, got {other:?}"),
    }
}

#[test]
fn test_facing_lasers() {
    let mut w = World::try_from(
        "
        @ @ L0S @  @
        @ F  .  S0 @
        @ .  .  .  @
        @ F  .  S1 @
        @ @ L1N  @ @",
    )
    .unwrap();
    w.reset();
    w.step(&[Action::West, Action::West]);
    assert!(!w.done());
    assert!(w.agents().iter().all(Agent::is_alive));
    let laser = w.get((2, 2)).unwrap().tile_type();
    match laser {
        TileType::Laser(l) => {
            assert!(l.is_off());
            let l2 = l.wrapped().tile_type();
            match l2 {
                TileType::Laser(l) => {
                    assert!(l.is_off());
                }
                other => panic!("Expected Laser, got {other:?}"),
            }
        }
        other => panic!("Expected Laser, got {other:?}"),
    }
}
