use core::panic;
use std::vec;

use crate::{
    Action, Grid, ParseError, Position, RuntimeWorldError, WorldEvent,
    agent::Agent,
    core::WorldState,
    tiles::{Button, Direction, Laser, Lift, Tile},
};

use super::World;

fn get_laser(world: &World, pos: Position) -> &Laser {
    for (p, laser) in world.lasers() {
        if pos == p {
            return laser;
        }
    }
    panic!("No laser at position {:?}", pos);
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
    assert!(world.start_positions.contains(&(Position::new2d(0, 0))));
    //let start = world.start_positions[0].get(&(0, 0)).unwrap();
    assert_eq!(world.start_positions[0], Position::new2d(0, 0));

    assert!(
        world
            .gems_positions
            .iter()
            .any(|pos| pos.i == 0 && pos.j == 2)
    );
    let source = world
        .sources()
        .iter()
        .find(|(Position { i, j, k: _ }, _)| *i == 1 && *j == 0)
        .unwrap()
        .1;
    assert_eq!(source.agent_id(), 0);
    let laser = get_laser(&world, Position::new2d(1, 1));
    assert_eq!(laser.agent_id(), 0);
    let n_exits_at_1_1 = world
        .exits
        .iter()
        .filter(|Position { i, j, k: _ }| *i == 1 && *j == 1)
        .count();
    assert!(n_exits_at_1_1 == 1);
    assert!(world.wall_positions.len() == 2);
    assert!(world.wall_positions.contains(&Position::new2d(1, 2)));
    assert!(world.wall_positions.contains(&Position::new2d(1, 0)));
}

#[test]
fn test_duplicate_start_pos() {
    match World::try_from("S0 S0 X X") {
        Ok(..) => panic!("Should not be able to build a world with duplicate start positions"),
        Err(err) => match err {
            ParseError::DuplicateStartTile { start1, start2, .. } => {
                assert_eq!(start1, Position::new2d(0, 0));
                assert_eq!(start2, Position::new2d(0, 1));
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
    assert_eq!(world.agents_positions, vec![(0, 1), (0, 0)]);
    assert_eq!(world.start_positions, vec![(0, 1), (0, 0)]);
}

#[test]
fn test_start_pos_order_lvl6() {
    let mut world = World::from_file("lvl6").unwrap();
    assert_eq!(world.start_positions.len(), 4);
    world.reset();
    for (id, pos) in world.starts().into_iter().enumerate() {
        assert_eq!(pos, (0, id + 4));
        assert_eq!(world.agents_positions[id], (0, id + 4));
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
    for pos in w.lasers_positions {
        assert_ne!(pos, (2, 1));
        assert_ne!(pos, (3, 1));
    }
}

#[test]
fn test_laser_blocked_on_reset() -> Result<(), RuntimeWorldError> {
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
    assert!(get_laser(&w, (1, 2).into()).is_on());
    assert!(get_laser(&w, (2, 2).into()).is_off());
    assert!(get_laser(&w, (3, 2).into()).is_off());
    Ok(())
}

#[test]
fn test_facing_lasers() -> Result<(), RuntimeWorldError> {
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
    assert!(w.agents().iter().all(Agent::is_alive));
    for (_, laser) in w.lasers() {
        assert!(laser.is_off());
    }
    Ok(())
}

#[test]
fn test_event_exit_when_staying() {
    let mut w = World::try_from(
        "S0 X .
         S1 . X",
    )
    .unwrap();
    w.reset();
    // The first time the agent exits, an event should be generated
    assert_eq!(1, w.step(&[Action::East, Action::Stay]).unwrap().len());
    // The second time, no event should be generated
    assert_eq!(0, w.step(&[Action::Stay, Action::Stay]).unwrap().len());
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
fn test_force_state_invalid_number_of_agents() {
    let mut w = World::try_from(
        "
        S0 . G
        X  . .
    ",
    )
    .unwrap();
    w.reset();
    let s = WorldState::new_alive(
        [Position::new2d(1, 2), Position::new2d(0, 0)].into(),
        [true].into(),
    );
    match w.set_state(&s) {
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
    let s = WorldState::new_alive([(1, 2).into()].into(), [true, false].into());
    match w.set_state(&s) {
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
/// In this map, if agent 0 is in (0, 2) and agent 1 is in (0, 3), agent 0 is blocking the laser.
/// When agent 1 leaves the tile and goes to the right, the laser should NOT activate.
fn test_complex_laser_blocking() -> Result<(), RuntimeWorldError> {
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

    let laser = get_laser(&w, (0, 3).into());
    assert!(laser.is_on());

    let state = WorldState::new_alive([(0, 2).into(), (0, 3).into()].into(), [false; 5].into());
    w.set_state(&state).unwrap();
    let laser = get_laser(&w, (0, 3).into());
    assert!(laser.is_off());
    assert!(w.agents().iter().all(Agent::is_alive));

    w.step(&[Action::Stay, Action::East]).unwrap();
    assert!(w.agents().iter().all(Agent::is_alive));
    let laser = get_laser(&w, (0, 3).into());
    assert!(laser.is_off());
    Ok(())
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
    let s = WorldState::new_alive([(0, 0).into()].into(), [].into());
    w.set_state(&s).unwrap();
    let actions = w.available_actions();
    assert_eq!(actions.len(), 1);
    assert_eq!(actions[0].len(), 4);
    assert!(actions[0].contains(&Action::South));
    assert!(actions[0].contains(&Action::Stay));
    assert!(actions[0].contains(&Action::East));
    assert!(actions[0].contains(&Action::Trigger));
}

#[test]
fn test_die_in_void() {
    let mut w = World::try_from("S0 V X").unwrap();
    w.reset();
    w.step(&[Action::East]).unwrap();
    assert!(w.agents[0].is_dead());
}

#[test]
fn test_num_gems_collected() {
    let mut world = World::try_from("S0 G X").unwrap();
    world.reset();
    assert_eq!(world.n_gems_collected(), 0);
    world.step(&[Action::East]).unwrap();
    assert_eq!(world.n_gems_collected(), 1);
    world.step(&[Action::Stay]).unwrap();
    assert_eq!(world.n_gems_collected(), 1);
    world.step(&[Action::East]).unwrap();
    assert_eq!(world.n_gems_collected(), 1);
}

#[test]
fn test_num_agents_arrived() {
    let mut world = World::try_from("S0 G X").unwrap();
    world.reset();
    assert_eq!(world.n_agents_arrived(), 0);
    world.step(&[Action::East]).unwrap();
    assert_eq!(world.n_agents_arrived(), 0);
    world.step(&[Action::Stay]).unwrap();
    assert_eq!(world.n_agents_arrived(), 0);
    world.step(&[Action::East]).unwrap();
    assert_eq!(world.n_agents_arrived(), 1);
}

#[test]
fn parse_inconsistent_row_lengths() {
    match World::try_from(
        "X S0 .
         . .",
    ) {
        Ok(_) => panic!("Should not be able to parse worlds with inconsistent row lengths"),
        Err(e) => match e {
            ParseError::Inconsistent3Dimensions {
                actual_n_dims,
                expected_n_dims,
                layer,
                ..
            } => {
                assert_eq!(actual_n_dims, (2, 1));
                assert_eq!(expected_n_dims, (3, 1));
                assert_eq!(layer, 0);
            }
            _ => panic!("Expected InconsistentDimensions, got {e:?}"),
        },
    }
}

#[test]
fn parse_inconsistent_size_between_layers() {
    match World::try_from(
        "X S0 .
         . . .
         ;
         . .
         . .",
    ) {
        Ok(_) => panic!("Should not be able to parse worlds with inconsistent row lengths"),
        Err(e) => match e {
            ParseError::Inconsistent3Dimensions {
                actual_n_dims,
                expected_n_dims,
                layer,
                ..
            } => {
                assert_eq!(actual_n_dims, (2, 2));
                assert_eq!(expected_n_dims, (3, 2));
                assert_eq!(layer, 1);
            }
            _ => panic!("Expected InconsistentDimensions, got {e:?}"),
        },
    }
}
#[test]
fn parse_inconsistent_size_between_layers2() {
    match World::try_from(
        "X S0 .
         . . .
         ;
         . . .
         . . .
         . . .
         ",
    ) {
        Ok(_) => panic!("Should not be able to parse worlds with inconsistent column lengths"),
        Err(e) => match e {
            ParseError::Inconsistent3Dimensions {
                actual_n_dims,
                expected_n_dims,
                layer,
                ..
            } => {
                assert_eq!(actual_n_dims, (3, 3));
                assert_eq!(expected_n_dims, (2, 3));
                assert_eq!(layer, 1);
            }
            _ => panic!("Expected InconsistentDimensions, got {e:?}"),
        },
    }
}
#[test]
fn parse_inconsistent_start_exit_tiles() {
    match World::try_from("S1 S0 X") {
        Ok(_) => panic!("Should not be able to parse worlds with #exit < #start"),
        Err(e) => match e {
            ParseError::NotEnoughExitTiles { n_exits, n_starts } => {
                assert_eq!(n_starts, 2);
                assert_eq!(n_exits, 1);
            }
            _ => panic!("Expected InconsistentNumberOfAgents, got {e:?}"),
        },
    }
}

#[test]
fn parse_no_agents() {
    match World::try_from(". . G") {
        Ok(_) => panic!("Should not be able to create worlds without agents"),
        Err(e) => match e {
            ParseError::NoAgents => {}
            _ => panic!("Expected NoAgents, got {e:?}"),
        },
    }
}

#[test]
fn test_vertex_conflict() {
    let mut w = World::try_from(
        "
    S0 X .
    .  . .
    S1 X .",
    )
    .unwrap();
    w.reset();
    w.step(&[Action::South, Action::North]).unwrap();
    let pos = w.agents_positions();
    assert_eq!(pos[0], (0, 0));
    assert_eq!(pos[1], (2, 0));
}

#[test]
fn test_reset() {
    let mut w = World::try_from("S0 G X").unwrap();
    for _ in 0..10 {
        w.reset();
        assert_eq!(w.agents_positions()[0], (0, 0));
        assert_eq!(
            w.step(&[Action::East]).unwrap(),
            vec![WorldEvent::GemCollected { agent_id: 0 }]
        );
        assert_eq!(
            w.step(&[Action::East]).unwrap(),
            vec![WorldEvent::AgentExit { agent_id: 0 }]
        );
    }
}

#[test]
fn test_standard_levels() {
    for level in 1..7 {
        let name = format!("level{}", level);
        World::from_file(&name).unwrap();
        let name = format!("lvl{}", level);
        World::from_file(&name).unwrap();
    }
}

#[test]
fn test_get_level() {
    for level in 1..7 {
        World::get_level(level).unwrap();
    }
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
    let s = WorldState::new_alive([(1, 2).into()].into(), [true].into());
    w.set_state(&s).unwrap();
    assert_eq!(w.agents_positions()[0], (1, 2));
    let gem = w.gems()[0];
    assert!(gem.is_collected());
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
    let s = WorldState::new_alive([(1, 0).into()].into(), [true].into());
    w.set_state(&s).unwrap();
    assert_eq!(w.agents_positions()[0], (1, 0));
    let gem = w.gems()[0];
    assert!(gem.is_collected());
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
        agents_positions: vec![(1, 0).into(), (1, 1).into()],
        gems_collected: vec![false],
        agents_alive: vec![true, false],
    };
    w.set_state(&s).unwrap();
    assert!(w.agents()[1].is_dead());
}

#[test]
fn test_no_exits() {
    let toml_content = r#"
world_string = """
. . S0 . S1 . . . . S2 
. . .  . .  . . . . S3 
. . .  . .  . . . . . 
. . .  . .  . . . . . 
"""
"#;
    match World::try_from(toml_content) {
        Err(ParseError::NotEnoughExitTiles { .. }) => {}
        Ok(..) => panic!("Should not be able to create a world without exits"),
        Err(other) => panic!("Expected NotEnoughExitTiles, got {other:?}"),
    }
}

#[test]
fn test_wrong_world_state() {
    let mut w = World::try_from(
        "
        S0 L0S X
        S1  .  X
    ",
    )
    .unwrap();
    w.reset();
    let s = WorldState::new_alive([(0, 0).into(), (1, 1).into()].into(), [].into());
    match w.set_state(&s) {
        Err(RuntimeWorldError::InvalidWorldState { .. }) => {}
        other => panic!("Expected InvalidAgentPosition, got {other:?}"),
    }
}

#[test]
/// This test was introduced because the agents were attributed start positions that
/// were forbidden. The reason was that thte positions were not re-ordered after the
/// agents were ordered by number of available start positions.
///
/// Test the worst case scenario for the position selection algorithm:
/// since the algorithm assigns a position to each agent from the one with the least
/// possible positions to the one with the most, the worst case scenario is when
/// the possible start positions of each agent overlap the the all the previous agents
/// (i.e. the agents with less available start positions).
fn test_random_start_positions() {
    let toml_config = r#"
width = 10
height = 10
exits = [{i_min=9}]
[[agents]]
start_positions = [{i=0, j=0}, {i=1, j=0}, {i=2, j=0}, {i=3, j=0}]

[[agents]]
start_positions = [{i=0, j=0}, {i=1, j=0}, {i=2, j=0}]

[[agents]]
start_positions = [{i=0, j=0}, {i=1, j=0}]

[[agents]]
start_positions = [{i=0, j=0}]
"#;
    // The start positions of the agents overlap but at not the same for every agent.
    let mut world = World::try_from(toml_config).unwrap();
    for _ in 0..1_000 {
        world.reset();
        let positions = world.agents_positions();
        assert_eq!(positions[0], Position::new2d(3, 0));
        assert_eq!(positions[1], Position::new2d(2, 0));
        assert_eq!(positions[2], Position::new2d(1, 0));
        assert_eq!(positions[3], Position::new2d(0, 0));
    }
}

#[test]
fn set_exits() {
    let mut world = World::try_from(
        "
        S0 . G
        X  . .
    ",
    )
    .unwrap();
    world.reset();
    assert!(world.exits_positions().len() == 1);
    assert!(world.exits_positions().contains(&Position::new2d(1, 0)));

    world
        .set_exit_positions(vec![(0, 1).into(), (1, 1).into()])
        .unwrap();
    assert_eq!(world.exits_positions().len(), 2);
    assert!(world.exits_positions().contains(&Position::new2d(0, 1)));
    assert!(world.exits_positions().contains(&Position::new2d(1, 1)));
}

#[test]
fn set_exits_events() {
    let mut world = World::try_from(
        "
        S0 . G
        X  . .
    ",
    )
    .unwrap();
    world.reset();

    world
        .set_exit_positions(vec![(0, 1).into(), (1, 1).into()])
        .unwrap();
    let events = world.step(&[Action::East]).unwrap();
    assert_eq!(events.len(), 1);
    assert_eq!(
        events
            .iter()
            .filter(|e| matches!(e, WorldEvent::AgentExit { .. }))
            .count(),
        1
    );
    assert_eq!(
        events
            .iter()
            .filter(|e| matches!(e, WorldEvent::GemCollected { .. }))
            .count(),
        0
    );
    assert_eq!(
        events
            .iter()
            .filter(|e| matches!(e, WorldEvent::AgentDied { .. }))
            .count(),
        0
    );
}

#[test]
fn set_exits_old_exit_inactive() {
    let mut world = World::try_from(
        "
        S0 . G
        X  . .
    ",
    )
    .unwrap();
    world.reset();

    world
        .set_exit_positions(vec![(0, 1).into(), (1, 1).into()])
        .unwrap();

    let events = world.step(&[Action::South]).unwrap();
    assert!(events.is_empty());
}

/// Builds a 1-row world directly (bypassing the text parser, which doesn't
/// support Lift/Button yet) with the given tiles placed on top of a floor,
/// and one agent per given start position.
fn build_lift_button_world(
    width: usize,
    tiles: Vec<(Position, Tile)>,
    starts: Vec<Position>,
) -> World {
    let mut grid = Grid::<Tile>::new(width, 1, 1).default_init();
    for (pos, tile) in tiles {
        grid.replace_at(&pos, tile);
    }
    let random_start_positions = starts.into_iter().map(|p| vec![p]).collect();
    World::new(
        grid,
        vec![],
        random_start_positions,
        vec![],
        vec![],
        vec![],
        vec![],
        vec![],
    )
}

fn floor() -> Tile {
    Tile::Floor { agent: None }
}

#[test]
fn test_button_pulses_lift_only_moves_same_group_occupant() {
    let button_pos = Position::new2d(0, 0);
    let lift1_pos = Position::new2d(0, 1);
    let dest1_pos = Position::new2d(0, 2);
    let lift2_pos = Position::new2d(0, 3);

    let tiles = vec![
        (button_pos, Tile::Button(Button::new(1))),
        (lift1_pos, Tile::Lift(Lift::new(Direction::East, None, 1))),
        (lift2_pos, Tile::Lift(Lift::new(Direction::East, None, 2))),
    ];
    let mut world = build_lift_button_world(5, tiles, vec![button_pos, lift1_pos, lift2_pos]);

    world
        .step(&[Action::Trigger, Action::Stay, Action::Stay])
        .unwrap();

    assert_eq!(world.agents_positions()[0], button_pos);
    assert_eq!(world.agents_positions()[1], dest1_pos);
    // Different group: must not be pulsed by the button.
    assert_eq!(world.agents_positions()[2], lift2_pos);
}

#[test]
fn test_lift_with_no_occupant_does_nothing() {
    let button_pos = Position::new2d(0, 0);
    let lift_pos = Position::new2d(0, 1);
    let tiles = vec![
        (button_pos, Tile::Button(Button::new(1))),
        (lift_pos, Tile::Lift(Lift::new(Direction::East, None, 1))),
    ];
    let mut world = build_lift_button_world(3, tiles, vec![button_pos]);

    let events = world.step(&[Action::Trigger]).unwrap();

    assert!(events.is_empty());
    assert_eq!(world.agents_positions()[0], button_pos);
}

#[test]
fn test_two_lifts_same_group_both_move() {
    let button_pos = Position::new2d(0, 0);
    let lift_a = Position::new2d(0, 1);
    let dest_a = Position::new2d(0, 2);
    let dest_b = Position::new2d(0, 3);
    let lift_b = Position::new2d(0, 4);

    let tiles = vec![
        (button_pos, Tile::Button(Button::new(9))),
        (lift_a, Tile::Lift(Lift::new(Direction::East, None, 9))),
        (lift_b, Tile::Lift(Lift::new(Direction::West, None, 9))),
    ];
    let mut world = build_lift_button_world(5, tiles, vec![button_pos, lift_a, lift_b]);

    world
        .step(&[Action::Trigger, Action::Stay, Action::Stay])
        .unwrap();

    assert_eq!(world.agents_positions()[1], dest_a);
    assert_eq!(world.agents_positions()[2], dest_b);
}

#[test]
fn test_colliding_lift_destinations_revert() {
    let button_pos = Position::new2d(0, 0);
    let lift_a = Position::new2d(0, 1);
    let lift_b = Position::new2d(0, 3);

    let tiles = vec![
        (button_pos, Tile::Button(Button::new(3))),
        (lift_a, Tile::Lift(Lift::new(Direction::East, None, 3))),
        (lift_b, Tile::Lift(Lift::new(Direction::West, None, 3))),
    ];
    let mut world = build_lift_button_world(4, tiles, vec![button_pos, lift_a, lift_b]);

    world
        .step(&[Action::Trigger, Action::Stay, Action::Stay])
        .unwrap();

    // Both lifts targeted position (0, 2): the conflict reverts both riders
    // back to where they stood (on their respective lifts).
    assert_eq!(world.agents_positions()[1], lift_a);
    assert_eq!(world.agents_positions()[2], lift_b);
}

#[test]
fn test_lift_destination_out_of_bounds_is_noop() {
    let button_pos = Position::new2d(0, 0);
    let lift_pos = Position::new2d(0, 1);
    let tiles = vec![
        (button_pos, Tile::Button(Button::new(1))),
        (lift_pos, Tile::Lift(Lift::new(Direction::East, None, 1))),
    ];
    // Width 2: lift sits on the last column, so East runs off the grid.
    let mut world = build_lift_button_world(2, tiles, vec![button_pos, lift_pos]);

    let events = world.step(&[Action::Trigger, Action::Stay]).unwrap();

    assert!(events.is_empty());
    assert_eq!(world.agents_positions()[1], lift_pos);
}

#[test]
fn test_lift_destination_wall_is_noop() {
    let button_pos = Position::new2d(0, 0);
    let lift_pos = Position::new2d(0, 1);
    let wall_pos = Position::new2d(0, 2);
    let tiles = vec![
        (button_pos, Tile::Button(Button::new(1))),
        (lift_pos, Tile::Lift(Lift::new(Direction::East, None, 1))),
        (wall_pos, Tile::Wall),
    ];
    let mut world = build_lift_button_world(3, tiles, vec![button_pos, lift_pos]);

    world.step(&[Action::Trigger, Action::Stay]).unwrap();

    assert_eq!(world.agents_positions()[1], lift_pos);
}

/// Builds a two-column, two-layer world (bypassing the text parser, which
/// doesn't support Lift/Button yet) with the given tiles placed, and one
/// agent per given start position. Two columns keep the button's cell
/// distinct from the lift's destination cell, so a floor-changing move
/// doesn't collide with the (stationary) button-standing agent.
fn build_multi_layer_lift_button_world(
    tiles: Vec<(Position, Tile)>,
    starts: Vec<Position>,
) -> World {
    let mut grid = Grid::<Tile>::new(2, 1, 2).default_init();
    for (pos, tile) in tiles {
        grid.replace_at(&pos, tile);
    }
    let random_start_positions = starts.into_iter().map(|p| vec![p]).collect();
    World::new(
        grid,
        vec![],
        random_start_positions,
        vec![],
        vec![],
        vec![],
        vec![],
        vec![],
    )
}

#[test]
fn test_button_pulses_lift_up_moves_agent_to_higher_layer() {
    let button_pos = Position { i: 0, j: 0, k: 0 };
    let lift_pos = Position { i: 0, j: 1, k: 0 };
    let dest_pos = Position { i: 0, j: 1, k: 1 };
    let tiles = vec![
        (button_pos, Tile::Button(Button::new(1))),
        (lift_pos, Tile::Lift(Lift::new(Direction::Up, None, 1))),
    ];
    let mut world = build_multi_layer_lift_button_world(tiles, vec![button_pos, lift_pos]);

    world.step(&[Action::Trigger, Action::Stay]).unwrap();

    assert_eq!(world.agents_positions()[0], button_pos);
    assert_eq!(world.agents_positions()[1], dest_pos);
}

#[test]
fn test_button_pulses_lift_down_moves_agent_to_lower_layer() {
    let button_pos = Position { i: 0, j: 0, k: 1 };
    let lift_pos = Position { i: 0, j: 1, k: 1 };
    let dest_pos = Position { i: 0, j: 1, k: 0 };
    let tiles = vec![
        (button_pos, Tile::Button(Button::new(1))),
        (lift_pos, Tile::Lift(Lift::new(Direction::Down, None, 1))),
    ];
    let mut world = build_multi_layer_lift_button_world(tiles, vec![button_pos, lift_pos]);

    world.step(&[Action::Trigger, Action::Stay]).unwrap();

    assert_eq!(world.agents_positions()[0], button_pos);
    assert_eq!(world.agents_positions()[1], dest_pos);
}
