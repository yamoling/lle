use std::{cell::Cell, rc::Rc};

use crate::{
    agent::Agent,
    tiles::{LaserBeam, Void},
    AgentId, Floor, Gem, Laser, Start, TeamReward, Tile,
};

fn make_laser(agent_id: AgentId, length: usize) -> Laser {
    let wrapped = Rc::new(Floor::default());
    let mut beam = Vec::with_capacity(4);
    (0..length).for_each(|_| beam.push(Rc::new(Cell::new(true))));
    let beam = LaserBeam::new(beam);
    let rm = Rc::new(TeamReward::new(1));
    Laser::new(agent_id, crate::tiles::Direction::East, wrapped, beam, rm)
}

#[test]
fn test_start() {
    let mut agent = Agent::new(0);
    let start = Start::new(agent.id());
    assert_eq!(start.agent_id(), 0);
    start.reset();
    assert_eq!(start.agent(), None);
    assert!(start.is_waklable());
    assert!(!start.is_occupied());
    assert_eq!(start.agent(), None);
    start.enter(&mut agent);
    assert_eq!(start.agent(), Some(agent.id()));
    assert!(start.is_occupied());
    start.leave();
    assert_eq!(start.agent(), None);
}

#[test]
fn test_gem() {
    let mut agent = Agent::new(3);
    let gem = Gem::new(Rc::new(TeamReward::new(3)));
    gem.reset();
    assert_eq!(gem.agent(), None);
    assert!(!gem.is_collected());
    assert!(gem.is_waklable());
    assert!(!gem.is_occupied());
    gem.collect();
    assert!(gem.is_collected());

    gem.reset();
    gem.enter(&mut agent);
    assert_eq!(gem.agent(), Some(agent.id()));
    assert!(gem.is_occupied());
    assert!(gem.is_collected());
    gem.leave();
    assert_eq!(gem.agent(), None);
}

#[test]
fn test_laser_basic() {
    let laser = make_laser(0, 4);
    laser.reset();
    assert_eq!(laser.agent_id(), 0);
    assert!(laser.is_waklable());
    assert!(!laser.is_occupied());
    assert!(laser.is_on());
}

#[test]
fn test_laser_agent_survives() {
    let mut agent = Agent::new(0);
    let laser = make_laser(0, 3);
    laser.pre_enter(&agent);
    assert!(!laser.is_occupied());
    assert!(laser.is_off());

    laser.enter(&mut agent);
    assert!(agent.is_alive());

    laser.leave();
    assert!(agent.is_alive());
    assert!(laser.is_on());
    assert!(!laser.is_occupied());
}

#[test]
fn test_laser_agent_dies() {
    let mut agent = Agent::new(0);
    let laser = make_laser(2, 3);
    laser.pre_enter(&agent);
    assert!(!laser.is_occupied());
    assert!(laser.is_on());
    assert!(agent.is_alive());

    laser.enter(&mut agent);
    assert!(agent.is_dead());
}

#[test]
fn test_void_agent_dies() {
    let mut agent = Agent::new(0);
    let collector = TeamReward::new(1);
    let void = Void::new(Rc::new(collector));
    void.pre_enter(&agent);
    assert!(!void.is_occupied());
    assert!(agent.is_alive());
    void.enter(&mut agent);
    assert!(agent.is_dead());
}
