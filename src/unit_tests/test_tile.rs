use std::sync::{atomic::AtomicBool, Arc, Mutex};

use crate::{
    agent::Agent,
    tiles::{Direction, Floor, Gem, Laser, LaserBeam, Start, Void},
    AgentId, Tile,
};

fn make_laser(agent_id: AgentId, length: usize) -> Laser {
    let wrapped = Arc::<Mutex<Floor>>::default();
    let mut beam = Vec::with_capacity(4);
    (0..length).for_each(|_| beam.push(Arc::new(AtomicBool::new(true))));
    let beam = Arc::new(Mutex::new(LaserBeam::new(
        length,
        agent_id,
        Direction::East,
        0,
    )));
    Laser::new(wrapped, beam, 0)
}

#[test]
fn test_start() {
    let mut agent = Agent::new(0);
    let mut start = Start::new(agent.id());
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
    let mut gem = Gem::default();
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
    let mut laser = make_laser(0, 4);
    laser.reset();
    assert_eq!(laser.agent_id(), 0);
    assert!(laser.is_waklable());
    assert!(!laser.is_occupied());
    assert!(laser.is_on());
}

#[test]
fn test_laser_agent_survives() {
    let mut agent = Agent::new(0);
    let mut laser = make_laser(0, 3);
    laser.pre_enter(&agent).unwrap();
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
    let mut laser = make_laser(2, 3);
    laser.pre_enter(&agent).unwrap();
    assert!(!laser.is_occupied());
    assert!(laser.is_on());
    assert!(agent.is_alive());

    laser.enter(&mut agent);
    assert!(agent.is_dead());
}

#[test]
fn test_void_agent_dies() {
    let mut agent = Agent::new(0);
    let mut void = Void::default();
    void.pre_enter(&agent).unwrap();
    assert!(!void.is_occupied());
    assert!(agent.is_alive());
    void.enter(&mut agent);
    assert!(agent.is_dead());
}

#[test]
fn laser_source_from_str() {
    use crate::tiles::LaserSource;
    let source = LaserSource::from_str("L0E", 0).unwrap().build().0;
    assert_eq!(source.direction(), crate::tiles::Direction::East);
    assert_eq!(source.agent_id(), 0);
    assert_eq!(source.laser_id(), 0);

    let source = LaserSource::from_str("L1W", 25).unwrap().build().0;
    assert_eq!(source.direction(), crate::tiles::Direction::West);
    assert_eq!(source.agent_id(), 1);
    assert_eq!(source.laser_id(), 25);

    let source = LaserSource::from_str("L2N", 0).unwrap().build().0;
    assert_eq!(source.direction(), crate::tiles::Direction::North);
    assert_eq!(source.agent_id(), 2);
    assert_eq!(source.laser_id(), 0);

    let source = LaserSource::from_str("L3S", 0).unwrap().build().0;
    assert_eq!(source.direction(), crate::tiles::Direction::South);
    assert_eq!(source.agent_id(), 3);
    assert_eq!(source.laser_id(), 0);
}
