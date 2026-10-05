use core::panic;
use std::vec;

use crate::{
    Action, ParseError, Position, RuntimeWorldError, WorldEvent, agent::Agent, core::WorldState,
    tiles::Laser,
};

use super::World;

fn pos(i: usize, j: usize) -> Position {
    Position { i, j }
}

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
    assert!(world.start_positions.contains(&(Position { i: 0, j: 0 })));
    //let start = world.start_positions[0].get(&(0, 0)).unwrap();
    assert_eq!(world.start_positions[0], Position { i: 0, j: 0 });

    assert!(
        world
            .gems_positions
            .iter()
            .any(|pos| pos.i == 0 && pos.j == 2)
    );
    let source = world
        .sources()
        .find(|(Position { i, j }, _)| *i == 1 && *j == 0)
        .unwrap()
        .1;
    assert_eq!(source.agent_id(), 0);
    let laser = get_laser(&world, Position { i: 1, j: 1 });
    assert_eq!(laser.agent_id(), 0);
    let n_exits_at_1_1 = world
        .exits
        .iter()
        .filter(|Position { i, j }| *i == 1 && *j == 1)
        .count();
    assert!(n_exits_at_1_1 == 1);
    assert!(world.wall_positions.len() == 2);
    assert!(world.wall_positions.contains(&Position { i: 1, j: 2 }));
    assert!(world.wall_positions.contains(&Position { i: 1, j: 0 }));
}

/// See `.agents/plans/agent-colour-id.md` §3.4: a repeated `S<c>` token no longer is a
/// `DuplicateStartTile` error, it declares several agents that share the colour `c`.
#[test]
fn test_repeated_start_token_declares_several_agents() {
    let world = World::try_from("S0 S0 X X").unwrap();
    assert_eq!(world.n_agents(), 2);
    assert_eq!(world.possible_starts()[0], vec![pos(0, 0)]);
    assert_eq!(world.possible_starts()[1], vec![pos(0, 1)]);
    // (That both agents have colour 0 is asserted in `test_agent_colour.rs`, which needs the
    // `Agent::colour` API.)
}

/// Agents are ordered by `(colour, reading order)`, never by reading order alone: the lone
/// colour-0 agent is id 0 even though it is written last. See §3.4a — a plain reading-order rule
/// would silently permute the agents of every level whose tokens are not in numeric order (cf.
/// `test_start_pos_order` below).
#[test]
fn test_agent_ids_are_colour_major() {
    let world = World::try_from("S1 S1 S0 X X X").unwrap();
    assert_eq!(world.n_agents(), 3);
    assert_eq!(world.possible_starts()[0], vec![pos(0, 2)]);
    assert_eq!(world.possible_starts()[1], vec![pos(0, 0)]);
    assert_eq!(world.possible_starts()[2], vec![pos(0, 1)]);
}

/// Gaps in the token numbering used to be a `ParseError::AgentWithoutStart`. Now that colour is
/// independent of agent count, they describe a sparse colour space (§3.4b).
#[test]
fn test_sparse_colour_tokens_are_legal() {
    let world = World::try_from("S0 S2 X X").unwrap();
    assert_eq!(world.n_agents(), 2);
}

/// The v1 format can express a shared-colour world, so `world_string()` must round-trip it
/// without falling back to TOML (§3.4d).
#[test]
fn test_repeated_tokens_round_trip_through_v1() {
    let world = World::try_from("S0 S0 X X").unwrap();
    let string = world.world_string();
    assert!(
        !string.contains("colour"),
        "Expected a v1 string, got:\n{string}"
    );
    let round_tripped = World::try_from(string).unwrap();
    assert_eq!(round_tripped.n_agents(), 2);
    assert_eq!(round_tripped.possible_starts(), world.possible_starts());
}

/// Two agents written as `S1` share colour 1, so neither dies on a colour-1 beam and either of
/// them switches it off. This is the core rule change (§3.1).
#[test]
fn test_same_colour_agents_are_immune_to_their_beam() {
    let mut world = World::try_from(
        "
    S1  .  .  X
    L1E .  .  .
    .   S1 .  X
    ",
    )
    .unwrap();
    world.reset();
    assert_eq!(world.n_agents(), 2);
    assert!(get_laser(&world, pos(1, 1)).is_on());

    // Agent 1 steps from (2, 1) onto the beam at (1, 1).
    let events = world.step(&[Action::Stay, Action::North]).unwrap();
    assert!(
        events.is_empty(),
        "An agent of the beam's colour must survive it, got {events:?}"
    );
    assert!(world.agents()[1].is_alive());
    assert!(
        get_laser(&world, pos(1, 1)).is_off(),
        "An agent of the beam's colour must switch the beam off"
    );
    assert!(get_laser(&world, pos(1, 2)).is_off());
}

/// Both same-colour agents stand on the same beam; when the upstream one leaves, the downstream
/// one keeps blocking on its own behalf.
#[test]
fn test_two_same_colour_agents_on_one_beam() {
    let mut world = World::try_from(
        "
    S1  .  .  X
    L1E .  .  .
    .   S1 .  X
    ",
    )
    .unwrap();
    world.reset();
    // Bring agent 0 to (0, 1) and agent 1 to (2, 2), then both onto the beam at once.
    world.step(&[Action::East, Action::East]).unwrap();
    let events = world.step(&[Action::South, Action::North]).unwrap();
    assert!(
        events.is_empty(),
        "Both agents share the colour: {events:?}"
    );
    assert_eq!(world.agents_positions(), &vec![pos(1, 1), pos(1, 2)]);

    let events = world.step(&[Action::North, Action::Stay]).unwrap();
    assert!(
        events.is_empty(),
        "The downstream agent blocks the beam itself: {events:?}"
    );
    assert!(world.agents()[1].is_alive());
    assert!(
        get_laser(&world, pos(1, 1)).is_on(),
        "The tile the blocker left must light up again"
    );
    assert!(get_laser(&world, pos(1, 2)).is_off());
    assert!(get_laser(&world, pos(1, 3)).is_off());
}

/// An agent whose colour differs from the beam's still dies. Control test: today's behaviour,
/// which must not change.
#[test]
fn test_different_colour_agent_still_dies_on_beam() {
    let mut world = World::try_from(
        "
    S0  .  .  X
    L0E .  .  .
    .   S1 .  X
    ",
    )
    .unwrap();
    world.reset();
    let events = world.step(&[Action::Stay, Action::North]).unwrap();
    assert!(
        events
            .iter()
            .any(|e| matches!(e, WorldEvent::AgentDied { agent_id: 1 })),
        "An agent of a different colour must die on an active beam, got {events:?}"
    );
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
        [Position { i: 1, j: 2 }, Position { i: 0, j: 0 }].into(),
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
            ParseError::InconsistentDimensions {
                actual_n_cols,
                expected_n_cols,
                row,
                ..
            } => {
                assert_eq!(actual_n_cols, 2);
                assert_eq!(expected_n_cols, 3);
                assert_eq!(row, 1);
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

/// A state round trip must restore gems covered by laser tiles.
#[test]
fn test_force_state_with_gem_under_laser() {
    let mut world = World::try_from(
        "
        .  G   X
        S0 L0N .
    ",
    )
    .unwrap();
    let state = WorldState::new_alive([(1, 0).into()].into(), [true].into());
    world.set_state(&state).unwrap();
    assert_eq!(world.get_state(), state);
    assert!(world.gems()[0].is_collected());
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
        assert_eq!(positions[0], Position { i: 3, j: 0 });
        assert_eq!(positions[1], Position { i: 2, j: 0 });
        assert_eq!(positions[2], Position { i: 1, j: 0 });
        assert_eq!(positions[3], Position { i: 0, j: 0 });
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
    assert!(world.exits_positions().contains(&Position { i: 1, j: 0 }));

    world
        .set_exit_positions(vec![(0, 1).into(), (1, 1).into()])
        .unwrap();
    assert_eq!(world.exits_positions().len(), 2);
    assert!(world.exits_positions().contains(&Position { i: 0, j: 1 }));
    assert!(world.exits_positions().contains(&Position { i: 1, j: 1 }));
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

#[test]
fn test_beam_single_source() {
    let mut w = World::try_from(
        "
        L0E . L0S
        S0  X  @",
    )
    .unwrap();
    w.reset();

    let src1 = w.source_at(pos(0, 0)).unwrap();
    let positions: Vec<Position> = w.beam(src1.laser_id()).unwrap().collect();
    assert_eq!(1, positions.len());
    assert!(positions.contains(&Position { i: 0, j: 1 }));

    let src2 = w.source_at(pos(0, 2)).unwrap();
    let positions: Vec<Position> = w.beam(src2.laser_id()).unwrap().collect();
    assert!(positions.is_empty());
}

#[test]
fn check_source_laser_id_is_the_same_as_source_index() {
    let w = World::try_from(
        "
S0  S1  S2  S3  S4  S5  S6  S7  S8  S9  S10
L0S L1S L2S L3S L4S L5S L6S L7S L8S L9S L10S
 .  .   .   .   .   .   .   .   .   .   .
 X  X   X   X   X   X   X   X   X   X   X
",
    )
    .unwrap();
    w.sources().enumerate().for_each(|(i, (_, source))| {
        assert_eq!(i, source.laser_id());
    });
}

#[test]
fn test_beam_long() {
    let mut w = World::try_from(
        "
        L0E .  .  .  .  @
        S0  .  .  .  X  @",
    )
    .unwrap();
    w.reset();

    let first = w.source_at((0, 0).into()).unwrap().laser_id();
    let positions: Vec<Position> = w.beam(first).unwrap().collect();
    assert_eq!(
        positions,
        vec![
            Position { i: 0, j: 1 },
            Position { i: 0, j: 2 },
            Position { i: 0, j: 3 },
            Position { i: 0, j: 4 },
        ]
    );
}

#[test]
fn test_beam_south_direction() {
    let mut w = World::try_from(
        "
        @  L0S @
        .  .   .
        .  .   .
        S0 X   @",
    )
    .unwrap();
    w.reset();

    let first = w.source_at((0, 1).into()).unwrap().laser_id();
    let positions: Vec<Position> = w.beam(first).unwrap().collect();
    assert_eq!(positions, vec![pos(1, 1), pos(2, 1), pos(3, 1)]);
}

#[test]
fn test_beam_agent_on_beam_tile() {
    let mut w = World::try_from(
        "
        L0E .  .  .
        .   S0 .  X
        .   .  .  . ",
    )
    .unwrap();
    w.reset();
    w.step(&[Action::North]).unwrap();
    assert_eq!(w.agents_positions()[0], Position { i: 0, j: 1 });

    let first = w.source_at((0, 0).into()).unwrap().laser_id();
    let positions: Vec<Position> = w.beam(first).unwrap().collect();
    assert_eq!(positions.len(), 3);
    assert_eq!(positions, vec![pos(0, 1), pos(0, 2), pos(0, 3)]);
}

#[test]
fn test_beam_two_sources() {
    let w = World::try_from(
        "
        @ L0E .  .  .
        .  .  .  .  .
        @ L1E .  @  @
        S0 .  .  X  .
        S1 .  .  X  .",
    )
    .unwrap();

    let l0_first = w.source_at((0, 1).into()).unwrap().laser_id();
    let l0_beam = w.beam(l0_first).unwrap();
    assert_eq!(l0_beam.count(), 3);
    let l1_first = w.source_at((2, 1).into()).unwrap().laser_id();
    let l1_beam = w.beam(l1_first).unwrap();
    assert_eq!(l1_beam.count(), 1);
}

#[test]
fn test_box_blocks_a_beam_from_reset() {
    let mut world = World::try_from(
        "
        L0S .  X
        B   .  .
        .   S0 .
        ",
    )
    .unwrap();
    world.reset();
    assert_eq!(world.n_boxes(), 1);
    assert_eq!(world.boxes_positions(), vec![pos(1, 0)]);
    assert_eq!(world.boxes_present(), vec![true]);
    assert!(
        get_laser(&world, pos(1, 0)).is_off(),
        "the box must block the beam where it stands"
    );
    assert!(
        get_laser(&world, pos(2, 0)).is_off(),
        "and downstream of it"
    );
}

#[test]
fn test_world_without_boxes_is_unaffected() {
    let world = World::try_from("S0 . X").unwrap();
    assert_eq!(world.n_boxes(), 0);
    assert!(world.boxes_positions().is_empty());
    assert!(!world.has_box_at(pos(0, 1)));
}

#[test]
fn test_has_box_at_tracks_the_occupancy_index() {
    let mut world = World::try_from("S0 B . X").unwrap();
    world.reset();
    assert!(world.has_box_at(pos(0, 1)));
    assert!(!world.has_box_at(pos(0, 2)));
    assert_eq!(world.box_id_at(pos(0, 1)), Some(0));
    assert_eq!(world.box_id_at(pos(0, 2)), None);
}

#[test]
fn test_an_agent_may_start_in_the_shadow_of_a_box() {
    // Beam runs West from (0,4). The box at (0,2) blocks it, so the agent's
    // only start at (0,1) is safe and must not be pruned away.
    let mut world = World::try_from("X S1 B . L0W").expect("the box shields the start");
    world.reset();
    assert_eq!(world.agents_positions(), &vec![pos(0, 1)]);
    assert!(world.agents()[0].is_alive());
    assert!(get_laser(&world, pos(0, 1)).is_off());
}

#[test]
fn test_beam_upstream_of_a_box_stays_on() {
    let mut world = World::try_from("L0E . B . X\nS1 . . . .").unwrap();
    world.reset();
    assert!(get_laser(&world, pos(0, 1)).is_on());
    assert!(get_laser(&world, pos(0, 2)).is_off());
    assert!(get_laser(&world, pos(0, 3)).is_off());
}

#[test]
fn test_reset_keeps_the_box_blocking_and_the_boxes_in_place() {
    let mut world = World::try_from("L0E B . X\nS1 . . .").unwrap();
    world.reset();
    world.reset();
    assert_eq!(world.boxes_positions(), vec![pos(0, 1)]);
    assert_eq!(world.boxes_present(), vec![true]);
    assert!(get_laser(&world, pos(0, 1)).is_off());
    assert!(get_laser(&world, pos(0, 2)).is_off());
}

#[test]
fn test_get_config_preserves_boxes() {
    let world = World::try_from("S0 B . X").unwrap();
    assert_eq!(world.get_config().boxes(), &vec![pos(0, 1)]);
}

#[test]
fn test_push_a_box_east() {
    let mut world = World::try_from("S0 B . X").unwrap();
    world.reset();
    assert!(world.available_actions()[0].contains(&Action::East));

    world.step(&[Action::East]).unwrap();

    assert_eq!(world.agents_positions(), &vec![pos(0, 1)]);
    assert_eq!(world.boxes_positions(), vec![pos(0, 2)]);
}

#[test]
fn test_push_blocked_by_a_wall() {
    let mut world = World::try_from("S0 B @ X\n.  . . .").unwrap();
    world.reset();
    assert!(!world.available_actions()[0].contains(&Action::East));
}

#[test]
fn test_push_blocked_by_another_box_no_chains() {
    let mut world = World::try_from("S0 B B . X").unwrap();
    world.reset();
    assert!(!world.available_actions()[0].contains(&Action::East));
}

#[test]
fn test_push_blocked_by_the_grid_edge() {
    let mut world = World::try_from("X . S0 B").unwrap();
    world.reset();
    assert!(!world.available_actions()[0].contains(&Action::East));
}

#[test]
fn test_push_blocked_by_a_living_agent() {
    let mut world = World::try_from("S0 B S1 X X").unwrap();
    world.reset();
    assert!(!world.available_actions()[0].contains(&Action::East));
}

#[test]
fn test_push_blocked_by_an_agent_that_died_in_a_beam() {
    // Beam runs East from (0,0) over (0,1); the box at (0,2) shields the rest.
    // Agent 1 (colour 1) walks North into the lit (0,1) and dies there. A
    // corpse in a lit beam is never recorded by the laser's wrapped tile, so
    // the push check must not rely on tile occupancy alone.
    let mut world = World::try_from(
        "
        L0E .  B  S0 .
        X   S1 .  .  X
        ",
    )
    .unwrap();
    world.reset();
    assert!(
        world.available_actions()[0].contains(&Action::West),
        "the push is legal while (0,1) is empty"
    );

    let events = world.step(&[Action::Stay, Action::North]).unwrap();
    assert!(events.contains(&WorldEvent::AgentDied { agent_id: 1 }));
    assert_eq!(world.agents_positions()[1], pos(0, 1), "the corpse stays");

    assert!(
        !world.available_actions()[0].contains(&Action::West),
        "a dead agent blocks the push"
    );
}

#[test]
fn test_push_blocked_by_an_agent_that_died_in_a_void() {
    let mut world = World::try_from(
        "
        S0 B  V  .
        X  .  S1 X
        ",
    )
    .unwrap();
    world.reset();
    assert!(world.available_actions()[0].contains(&Action::East));

    let events = world.step(&[Action::Stay, Action::North]).unwrap();
    assert!(events.contains(&WorldEvent::AgentDied { agent_id: 1 }));

    assert!(
        !world.available_actions()[0].contains(&Action::East),
        "a dead agent blocks the push"
    );
}

#[test]
fn test_push_blocked_by_an_arrived_agent() {
    let mut world = World::try_from(
        "
        S0 B X
        X  . S1
        ",
    )
    .unwrap();
    world.reset();
    assert!(world.available_actions()[0].contains(&Action::East));

    let events = world.step(&[Action::Stay, Action::North]).unwrap();
    assert!(events.contains(&WorldEvent::AgentExit { agent_id: 1 }));
    assert!(world.agents()[1].has_arrived());

    assert!(
        !world.available_actions()[0].contains(&Action::East),
        "an arrived agent blocks the push"
    );
}

#[test]
fn test_a_box_may_rest_on_an_exit() {
    // Two exits, one agent: pushing a box onto an exit is legal and the
    // obstruction is reversible, so it is not a deadlock.
    let mut world = World::try_from("S0 B X X").unwrap();
    world.reset();
    assert!(world.available_actions()[0].contains(&Action::East));

    world.step(&[Action::East]).unwrap();

    assert_eq!(world.boxes_positions(), vec![pos(0, 2)]);
}

/// The ordering case. The beam points West from the source at (0,4) across
/// (0,3) offset 0, (0,2) offset 1, (0,1) offset 2, (0,0) offset 3. The box at
/// (0,2) already blocks from offset 1, so the agent's start at (0,1) is dark.
///
/// Moving East pushes the box to (0,3) — offset 0 — darkening the entire beam,
/// and the agent lands on (0,2). Phase 2 releases every blocker, so if agents
/// entered before boxes settled, the agent would walk into a lit beam and die.
/// Phase 3 must precede phases 4 and 5.
#[test]
fn test_a_pushed_box_saves_its_pusher() {
    let mut world = World::try_from("X S1 B . L0W").unwrap();
    world.reset();
    assert!(
        get_laser(&world, pos(0, 3)).is_on(),
        "offset 0 is lit at reset"
    );
    assert!(
        get_laser(&world, pos(0, 1)).is_off(),
        "the box shields the start"
    );

    let events = world.step(&[Action::East]).unwrap();

    assert!(events.is_empty(), "the agent must not die: {events:?}");
    assert_eq!(world.agents_positions(), &vec![pos(0, 2)]);
    assert_eq!(world.boxes_positions(), vec![pos(0, 3)]);
    assert!(world.agents()[0].is_alive());
    assert!(
        get_laser(&world, pos(0, 3)).is_off(),
        "the box now blocks offset 0"
    );
}

#[test]
fn test_two_agents_pushing_one_box_both_revert() {
    // Agent 0 at (0,1) pushes South, agent 1 at (1,0) pushes East.
    // Both destinations are (1,1), the box's cell -> vertex conflict.
    let mut world = World::try_from(
        "
        .  S0 X
        S1 B  X
        .  .  .
        ",
    )
    .unwrap();
    world.reset();

    world.step(&[Action::South, Action::East]).unwrap();

    assert_eq!(
        world.agents_positions(),
        &vec![pos(0, 1), pos(1, 0)],
        "both agents revert on the vertex conflict"
    );
    assert_eq!(
        world.boxes_positions(),
        vec![pos(1, 1)],
        "the box does not move"
    );
}

#[test]
fn test_agent_entering_a_pushed_box_destination_reverts_with_the_pusher() {
    // Agent 0 pushes the box from (0,1) to (0,2) while agent 1 walks North
    // into (0,2). Two occupants would share one cell: both agents revert.
    let mut world = World::try_from(
        "
        S0 B  .  X
        .  .  S1 X
        ",
    )
    .unwrap();
    world.reset();
    assert!(world.available_actions()[0].contains(&Action::East));
    assert!(world.available_actions()[1].contains(&Action::North));

    world.step(&[Action::East, Action::North]).unwrap();

    assert_eq!(world.agents_positions(), &vec![pos(0, 0), pos(1, 2)]);
    assert_eq!(
        world.boxes_positions(),
        vec![pos(0, 1)],
        "the box does not move"
    );
}

#[test]
fn test_two_boxes_pushed_into_one_cell_both_pushers_revert() {
    let mut world = World::try_from(
        "
        S0 B  .  B  S1
        X  .  .  .  X
        ",
    )
    .unwrap();
    world.reset();
    assert!(world.available_actions()[0].contains(&Action::East));
    assert!(world.available_actions()[1].contains(&Action::West));

    world.step(&[Action::East, Action::West]).unwrap();

    assert_eq!(world.agents_positions(), &vec![pos(0, 0), pos(0, 4)]);
    assert_eq!(
        world.boxes_positions(),
        vec![pos(0, 1), pos(0, 3)],
        "neither box moves"
    );
}

#[test]
fn test_box_contention_reverts_only_the_agents_involved() {
    // Agent 0 pushes the box into (0,2) while agent 1 walks into (0,2): both
    // revert. Agent 2 is uninvolved and still moves. (Agents never walk into an
    // occupied cell, so a revert cannot cascade onto a follower.)
    let mut world = World::try_from(
        "
        S0 B  .  X
        S2 .  S1 X
        .  .  .  X
        ",
    )
    .unwrap();
    world.reset();
    assert!(
        !world.available_actions()[2].contains(&Action::North),
        "agents cannot follow into an occupied cell"
    );

    world
        .step(&[Action::East, Action::North, Action::South])
        .unwrap();

    assert_eq!(
        world.agents_positions(),
        &vec![pos(0, 0), pos(1, 2), pos(2, 0)]
    );
    assert_eq!(world.boxes_positions(), vec![pos(0, 1)]);
}

#[test]
fn test_uncontended_pushes_in_the_same_step_all_happen() {
    // Two agents push two different boxes to two different cells.
    let mut world = World::try_from(
        "
        S0 B  .  .  X
        S1 B  .  .  X
        ",
    )
    .unwrap();
    world.reset();

    world.step(&[Action::East, Action::East]).unwrap();

    assert_eq!(world.agents_positions(), &vec![pos(0, 1), pos(1, 1)]);
    assert_eq!(world.boxes_positions(), vec![pos(0, 2), pos(1, 2)]);
}

#[test]
fn test_box_pushed_into_a_beam_blocks_it() {
    // Source at (0,2) fires South over (1,2) offset 0 and (2,2) offset 1.
    // The box starts outside the beam at (1,1); the agent pushes it in.
    let mut world = World::try_from(
        "
        .  .  L0S
        S1 B  X
        .  .  X
        ",
    )
    .unwrap();
    world.reset();
    assert!(get_laser(&world, pos(1, 2)).is_on());
    assert!(get_laser(&world, pos(2, 2)).is_on());

    world.step(&[Action::East]).unwrap();

    assert_eq!(world.boxes_positions(), vec![pos(1, 2)]);
    assert!(get_laser(&world, pos(1, 2)).is_off());
    assert!(get_laser(&world, pos(2, 2)).is_off(), "and downstream");
}

#[test]
fn test_box_pushed_out_of_a_beam_relights_it_and_kills() {
    // Source at (0,1) fires South over (1,1) offset 0 and (2,1) offset 1.
    // The box starts in the beam at (1,1), shielding nothing the agent needs.
    // Pushing it East to (1,2) relights the beam onto the agent's destination.
    let mut world = World::try_from(
        "
        .  L0S .  X
        S1 B   .  X
        .  .   .  .
        ",
    )
    .unwrap();
    world.reset();
    assert!(
        get_laser(&world, pos(1, 1)).is_off(),
        "the box blocks at reset"
    );

    let events = world.step(&[Action::East]).unwrap();

    assert_eq!(world.boxes_positions(), vec![pos(1, 2)]);
    assert!(
        events.contains(&WorldEvent::AgentDied { agent_id: 0 }),
        "the relit beam kills the agent that walked into it: {events:?}"
    );
    assert!(!world.agents()[0].is_alive());
}

#[test]
fn test_death_replay_neither_moves_nor_destroys_a_box_twice() {
    // The box is pushed into the void while agent 1 dies in a beam on the
    // same step, which forces the death-replay pass. The replay re-settles
    // boxes but must neither move the box again nor re-emit its destruction.
    let mut world = World::try_from(
        "
        S0 B  V  L0S
        X  .  S1 .
        X  .  .  .
        ",
    )
    .unwrap();
    world.reset();

    let events = world.step(&[Action::East, Action::East]).unwrap();

    assert!(events.contains(&WorldEvent::AgentDied { agent_id: 1 }));
    let destroyed = events
        .iter()
        .filter(|e| matches!(e, WorldEvent::BoxDestroyed { .. }))
        .count();
    assert_eq!(destroyed, 1, "{events:?}");
    assert_eq!(world.boxes_present(), vec![false]);
    assert_eq!(world.boxes_positions(), vec![pos(0, 2)]);
    assert_eq!(world.agents_positions()[0], pos(0, 1));
}

#[test]
fn test_death_replay_does_not_move_a_pushed_box_twice() {
    // Same replay, but the box survives: it must advance exactly one cell.
    let mut world = World::try_from(
        "
        S0 B  .  .  L0S
        X  .  .  S1 .
        X  .  .  .  .
        ",
    )
    .unwrap();
    world.reset();

    let events = world.step(&[Action::East, Action::East]).unwrap();

    assert!(events.contains(&WorldEvent::AgentDied { agent_id: 1 }));
    assert_eq!(world.boxes_positions(), vec![pos(0, 2)]);
    assert_eq!(world.boxes_present(), vec![true]);
}

#[test]
fn test_box_pushed_onto_a_void_is_destroyed() {
    let mut world = World::try_from("S0 B V X").unwrap();
    world.reset();
    assert!(world.available_actions()[0].contains(&Action::East));

    let events = world.step(&[Action::East]).unwrap();

    assert_eq!(events, vec![WorldEvent::BoxDestroyed { box_id: 0 }]);
    assert_eq!(world.boxes_present(), vec![false]);
    assert!(
        !world.has_box_at(pos(0, 2)),
        "a destroyed box holds no cell"
    );
}

#[test]
fn test_a_destroyed_box_on_a_void_under_a_beam_does_not_block() {
    // Source at (0,2) fires South over the void at (1,2) offset 0 and (2,2)
    // offset 1 — a beam crosses a void, because `Void::is_walkable()` is true.
    // The agent at (1,0) pushes the box from (1,1) onto that void.
    let mut world = World::try_from(
        "
        .  .  L0S X
        S1 B  V   .
        .  .  .   .
        ",
    )
    .unwrap();
    world.reset();
    assert!(get_laser(&world, pos(1, 2)).is_on());

    let events = world.step(&[Action::East]).unwrap();

    assert!(
        events.contains(&WorldEvent::BoxDestroyed { box_id: 0 }),
        "the box falls into the void: {events:?}"
    );
    assert_eq!(world.boxes_present(), vec![false]);
    assert!(
        get_laser(&world, pos(1, 2)).is_on(),
        "a destroyed box must not block the beam it fell through"
    );
    assert!(
        get_laser(&world, pos(2, 2)).is_on(),
        "nor anything downstream of it"
    );
    assert_eq!(world.agents_positions(), &vec![pos(1, 1)]);
}

#[test]
fn test_reset_revives_a_destroyed_box() {
    let mut world = World::try_from("S0 B V X").unwrap();
    world.reset();
    world.step(&[Action::East]).unwrap();
    assert_eq!(world.boxes_present(), vec![false]);

    world.reset();

    assert_eq!(world.boxes_present(), vec![true]);
    assert_eq!(world.boxes_positions(), vec![pos(0, 1)]);
}

#[test]
fn test_box_on_a_gem_does_not_collect_it_and_frees_it_again() {
    let mut world = World::try_from("S0 B G . X").unwrap();
    world.reset();

    world.step(&[Action::East]).unwrap();
    assert_eq!(world.boxes_positions(), vec![pos(0, 2)]);
    assert_eq!(world.n_gems_collected(), 0, "a box never collects a gem");

    world.step(&[Action::East]).unwrap();
    assert_eq!(world.boxes_positions(), vec![pos(0, 3)]);
    assert_eq!(
        world.n_gems_collected(),
        1,
        "the agent collects it after the push"
    );
}

#[test]
fn test_a_box_on_an_exit_obstructs_it() {
    // The box rests on the exit and has nowhere to go: the agent cannot enter.
    let mut world = World::try_from("S0 B X @").unwrap();
    world.reset();
    world.step(&[Action::East]).unwrap();
    assert_eq!(world.boxes_positions(), vec![pos(0, 2)]);
    assert_eq!(world.agents_positions(), &vec![pos(0, 1)]);

    assert!(
        !world.available_actions()[0].contains(&Action::East),
        "the exit is blocked by a box that cannot be pushed further"
    );
    assert!(world.step(&[Action::East]).is_err());
    assert_eq!(world.agents_positions(), &vec![pos(0, 1)]);
}

#[test]
fn test_an_exit_works_again_once_its_box_is_pushed_off() {
    let mut world = World::try_from("S0 B X .").unwrap();
    world.reset();
    world.step(&[Action::East]).unwrap();
    assert_eq!(world.boxes_positions(), vec![pos(0, 2)]);

    let events = world.step(&[Action::East]).unwrap();

    assert_eq!(world.boxes_positions(), vec![pos(0, 3)]);
    assert_eq!(world.agents_positions(), &vec![pos(0, 2)]);
    assert!(
        events.contains(&WorldEvent::AgentExit { agent_id: 0 }),
        "the agent exits once the box is off the exit: {events:?}"
    );
}
