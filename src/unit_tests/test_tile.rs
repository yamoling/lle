use std::rc::Rc;

use crate::{
    AgentId, Tile,
    agent::Agent,
    tiles::{Direction, Gem, Laser, LaserBeam, Void},
};

fn make_laser(agent_id: AgentId, length: usize) -> Laser {
    let wrapped = Tile::Floor { agent: None };
    let beam = Rc::new(LaserBeam::new(length, agent_id, Direction::East, 0));
    Laser::new(wrapped, beam, 0)
}

#[test]
fn test_gem() {
    let mut agent = Agent::new(3);
    let mut tile = Tile::Gem(Gem::default());
    tile.reset();
    assert_eq!(tile.agent(), None);
    if let Tile::Gem(g) = &tile {
        assert!(!g.is_collected());
    }
    assert!(tile.is_walkable());
    assert!(!tile.is_occupied());

    // Enter the tile to collect the gem
    tile.enter(&mut agent);
    if let Tile::Gem(gem) = &tile {
        assert!(gem.is_collected());
    } else {
        panic!();
    }
    assert!(tile.is_occupied());
    tile.leave();
    assert_eq!(tile.agent(), None);
    if let Tile::Gem(gem) = &tile {
        assert!(gem.is_collected());
    } else {
        panic!()
    }

    // Reset the tile
    tile.reset();
    if let Tile::Gem(gem) = &tile {
        assert!(!gem.is_collected());
    } else {
        panic!()
    }
    assert_eq!(tile.agent(), None);
    assert!(!tile.is_occupied());
}

#[test]
fn test_laser_basic() {
    let laser = make_laser(0, 4);
    assert_eq!(laser.agent_id(), 0);
    assert!(laser.is_on());

    let mut laser = Tile::Laser(laser);
    laser.reset();
    assert!(laser.is_walkable());
    assert!(!laser.is_occupied());
}

#[test]
fn test_laser_agent_survives() {
    let mut agent = Agent::new(0);
    let laser = make_laser(0, 3);
    assert!(laser.is_on());
    let mut tile = Tile::Laser(laser);

    tile.pre_enter(&agent).unwrap();
    assert!(!tile.is_occupied());

    tile.enter(&mut agent);
    assert!(agent.is_alive());

    tile.leave();
    assert!(agent.is_alive());
    if let Tile::Laser(laser) = &tile {
        assert!(laser.is_on());
    }
    assert!(!tile.is_occupied());
}

#[test]
fn test_laser_agent_dies() {
    let mut agent = Agent::new(0);
    let mut laser = make_laser(2, 3);
    laser.pre_enter(&agent).unwrap();
    assert!(laser.is_on());
    assert!(agent.is_alive());

    laser.enter(&mut agent);
    assert!(agent.is_dead());
}

#[test]
fn test_void_agent_dies() {
    let mut agent = Agent::new(0);
    let mut void = Void::default();
    assert!(agent.is_alive());
    void.enter(&mut agent);
    assert!(agent.is_dead());
}
