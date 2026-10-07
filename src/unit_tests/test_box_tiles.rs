use std::rc::Rc;

use crate::{
    agent::Colour,
    tiles::{BoxOutcome, CardinalDirection, Gem, Laser, LaserBeam, Tile, Void},
};

fn beam(size: usize) -> Rc<LaserBeam> {
    Rc::new(LaserBeam::new(
        size,
        0 as Colour,
        CardinalDirection::East,
        0,
    ))
}

#[test]
fn test_box_rests_on_plain_tiles() {
    for mut tile in [
        Tile::Floor { agent: None },
        Tile::Exit { agent: None },
        Tile::Gem(Gem::default()),
    ] {
        assert_eq!(tile.box_enter(), BoxOutcome::Rests);
        tile.box_leave();
    }
}

#[test]
fn test_box_is_destroyed_on_void() {
    let mut tile = Tile::Void(Void::default());
    assert_eq!(tile.box_enter(), BoxOutcome::Destroyed);
}

#[test]
fn test_box_does_not_collect_a_gem() {
    let mut tile = Tile::Gem(Gem::default());
    assert_eq!(tile.box_enter(), BoxOutcome::Rests);
    match &tile {
        Tile::Gem(gem) => assert!(!gem.is_collected()),
        _ => panic!("expected a gem"),
    }
}

#[test]
fn test_box_turns_a_beam_off_and_back_on() {
    let b = beam(3);
    let mut tile = Tile::Laser(Laser::new(Tile::Floor { agent: None }, b.clone(), 1));
    assert!(b.is_on(1));

    assert_eq!(tile.box_enter(), BoxOutcome::Rests);
    assert!(b.is_off(1), "a box must turn the beam off");
    assert!(b.is_off(2), "and keep it off downstream");

    tile.box_leave();
    assert!(b.is_on(1), "the beam must relight when the box leaves");
}

/// A beam crosses a void, because `Void::is_walkable()` is true and `laser_setup`
/// runs straight through it. A box pushed there is destroyed, so it must NOT block.
#[test]
fn test_box_destroyed_on_a_void_under_a_beam_does_not_block() {
    let b = beam(3);
    let mut tile = Tile::Laser(Laser::new(Tile::Void(Void::default()), b.clone(), 1));

    assert_eq!(tile.box_enter(), BoxOutcome::Destroyed);
    assert!(b.is_on(1), "a destroyed box must not block the beam");
}
