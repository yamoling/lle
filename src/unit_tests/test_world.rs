use crate::{
    agent::Agent,
    tiles::{Exit, Floor, Gem, Laser, LaserSource, Start, Tile, Wall},
    Action, World, WorldError,
};

trait TestTile {
    fn as_laser(&self) -> &Laser;
    fn as_floor(&self) -> &Floor;
    fn as_wall(&self) -> &Wall;
    fn as_laser_source(&self) -> &LaserSource;
    fn as_gem(&self) -> &Gem;
    fn as_start(&self) -> &Start;
    fn as_exit(&self) -> &Exit;
}

impl TestTile for dyn Tile {
    fn as_laser(&self) -> &Laser {
        self.as_any().downcast_ref().unwrap()
    }
    fn as_floor(&self) -> &Floor {
        self.as_any().downcast_ref().unwrap()
    }
    fn as_wall(&self) -> &Wall {
        self.as_any().downcast_ref().unwrap()
    }
    fn as_laser_source(&self) -> &LaserSource {
        self.as_any().downcast_ref().unwrap()
    }
    fn as_gem(&self) -> &Gem {
        self.as_any().downcast_ref().unwrap()
    }
    fn as_start(&self) -> &Start {
        self.as_any().downcast_ref().unwrap()
    }
    fn as_exit(&self) -> &Exit {
        self.as_any().downcast_ref().unwrap()
    }
}

#[test]
fn test_tile_type() {
    let w = World::try_from(
        "
    S0 . G
    L0E X .
    ",
    )
    .unwrap();
    assert!(w.start_tiles.contains(&(0, 0)));
    let start: &Start = w.grid[0][0].as_start();
    assert_eq!(start.agent_id(), 0);
    let _floor: &Floor = w.grid[0][1].as_floor();
    let _gem: &Gem = w.grid[0][2].as_gem();
    let source: &LaserSource = w.grid[1][0].as_laser_source();
    assert_eq!(source.agent_id(), 0);
    let laser = w.grid[1][1].as_laser();
    assert_eq!(laser.agent_id(), 0);
    laser.wrapped().as_any().downcast_ref::<Exit>().unwrap();
    let laser: &Laser = w.grid[1][2].as_laser();
    assert_eq!(laser.agent_id(), 0);
    let _: &Floor = laser.wrapped().as_any().downcast_ref().unwrap();
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
    let l1 = w.grid[1][2].as_laser();
    let l2 = w.grid[2][2].as_laser();
    let l3 = w.grid[3][2].as_laser();
    assert!(l1.is_on());
    assert!(l2.is_off());
    assert!(l3.is_off());
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
        let l1: &Laser = w.grid[i][2].as_laser();
        assert!(l1.is_off());
        let l2: &Laser = l1.wrapped().as_any().downcast_ref().unwrap();
        let _: &Floor = l2.wrapped().as_any().downcast_ref().unwrap();
    }
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
