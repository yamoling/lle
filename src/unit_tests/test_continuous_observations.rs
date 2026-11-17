use crate::{
    continuous::{ContinuousWorld, ContinuousWorldState, LaserSourceInfo, PhysicsParams},
    shared::{CardinalDirection, MapSpec, Position},
};

/// Two agents of colours 0 and 1, one gem and one source of colour 0 (`n_colours = 2`).
///
/// ```text
/// S0 .   G .  X
/// S1 .   . .  X
/// @  L0N . .  .
/// ```
const MAP: &str = "S0 . G . X\nS1 . . . X\n@ L0N . . .";
const WIDTH: f32 = 5.0;
const HEIGHT: f32 = 3.0;

fn world() -> ContinuousWorld {
    ContinuousWorld::try_from(MAP).unwrap()
}

fn pos(i: usize, j: usize) -> Position {
    Position { i, j, k: 0 }
}

/// A state where agent 0 moves and agent 1 has arrived, the gem is collected and the source is
/// disabled.
fn custom_state() -> ContinuousWorldState {
    ContinuousWorldState {
        positions: vec![[1.5, 0.5], [4.5, 1.25]],
        velocities: vec![[1.0, -0.5], [0.0, 0.0]],
        gems_collected: vec![true],
        agents_alive: vec![true, true],
        agents_arrived: vec![false, true],
        sources_enabled: vec![false],
        tick: 7,
    }
}

#[test]
fn test_sizes_are_known_after_construction() {
    let world = world();
    assert_eq!(world.n_colours(), 2);
    assert_eq!(world.agent_block_size(), 8);
    // 2 agent blocks, 1 gem, 1 source.
    assert_eq!(world.state_size(), 2 * 8 + 1 + 1);
    assert_eq!(world.observation_size(), world.state_size());
    assert_eq!(world.state_vector().len(), world.state_size());
    for agent in 0..world.n_agents() {
        assert_eq!(
            world.observation(agent).unwrap().len(),
            world.observation_size()
        );
    }
}

#[test]
fn test_state_vector_at_reset() {
    let world = world();
    #[rustfmt::skip]
    let expected = vec![
        // Agent 0 at the centre of (0, 0), colour 0.
        0.5 / WIDTH, 0.5 / HEIGHT, 0.0, 0.0, 1.0, 0.0, 1.0, 0.0,
        // Agent 1 at the centre of (1, 0), colour 1.
        0.5 / WIDTH, 1.5 / HEIGHT, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0,
        // Gem not collected, source enabled.
        0.0, 1.0,
    ];
    assert_eq!(world.state_vector(), expected);
}

#[test]
fn test_state_vector_layout() {
    let mut world = world();
    world.set_state(&custom_state()).unwrap();
    let max_speed = PhysicsParams::default().max_speed as f32;
    #[rustfmt::skip]
    let expected = vec![
        1.5 / WIDTH, 0.5 / HEIGHT, 1.0 / max_speed, -0.5 / max_speed, 1.0, 0.0, 1.0, 0.0,
        4.5 / WIDTH, 1.25 / HEIGHT, 0.0, 0.0, 1.0, 1.0, 0.0, 1.0,
        1.0, 0.0,
    ];
    assert_eq!(world.state_vector(), expected);
}

#[test]
fn test_observation_is_ego_first_with_relative_positions() {
    let mut world = world();
    world.set_state(&custom_state()).unwrap();
    let max_speed = PhysicsParams::default().max_speed as f32;
    let state = world.state_vector();
    // Agent 0 observes itself first: its observation is the state with relative coordinates
    // for agent 1.
    let mut expected_0 = state.clone();
    expected_0[8] = (4.5 - 1.5) / WIDTH;
    expected_0[9] = (1.25 - 0.5) / HEIGHT;
    assert_eq!(world.observation(0), Some(expected_0));

    #[rustfmt::skip]
    let expected_1 = vec![
        // Agent 1 first, absolute.
        4.5 / WIDTH, 1.25 / HEIGHT, 0.0, 0.0, 1.0, 1.0, 0.0, 1.0,
        // Agent 0, relative to agent 1.
        (1.5 - 4.5) / WIDTH, (0.5 - 1.25) / HEIGHT, 1.0 / max_speed, -0.5 / max_speed, 1.0, 0.0, 1.0, 0.0,
        // Global block.
        1.0, 0.0,
    ];
    assert_eq!(world.observation(1), Some(expected_1));
}

#[test]
fn test_dead_agent_flag() {
    let mut world = world();
    let mut state = custom_state();
    state.agents_alive[0] = false;
    world.set_state(&state).unwrap();
    assert_eq!(world.state_vector()[4], 0.0);
    assert_eq!(world.observation(1).unwrap()[8 + 4], 0.0);
}

#[test]
fn test_observation_of_unknown_agent_is_none() {
    assert_eq!(world().observation(2), None);
}

#[test]
fn test_static_getters() {
    let world = world();
    assert_eq!(world.params(), PhysicsParams::default());
    assert_eq!(world.gems_positions(), vec![pos(0, 2)]);
    assert_eq!(world.exits_positions(), vec![pos(0, 4), pos(1, 4)]);
    assert_eq!(world.agents_colours(), vec![0, 1]);
    assert_eq!(
        world.laser_sources(),
        &[LaserSourceInfo {
            position: pos(2, 1),
            direction: CardinalDirection::North,
            colour: 0,
            laser_id: 0,
        }]
    );
    assert_eq!(world.beam_cells(), &[vec![pos(1, 1), pos(0, 1)]]);
}

#[test]
fn test_beam_cells_stop_at_walls_and_ignore_agents() {
    let world = ContinuousWorld::try_from("L0E S0 . @ X").unwrap();
    assert_eq!(world.beam_cells(), &[vec![pos(0, 1), pos(0, 2)]]);
}

#[test]
fn test_dynamic_getters() {
    let mut world = world();
    assert_eq!(world.agents_cells(), vec![pos(0, 0), pos(1, 0)]);
    assert_eq!(world.gems_collected(), vec![false]);
    assert_eq!(world.agents_alive(), vec![true, true]);
    assert_eq!(world.agents_arrived(), vec![false, false]);
    world.set_state(&custom_state()).unwrap();
    assert_eq!(world.agents_cells(), vec![pos(0, 1), pos(1, 4)]);
    assert_eq!(world.gems_collected(), vec![true]);
    assert_eq!(world.agents_arrived(), vec![false, true]);
}

#[test]
fn test_agents_cells_at_cell_boundaries() {
    let mut world = world();
    let mut state = custom_state();
    // On the boundary between two cells, the centre belongs to the cell on the right / below.
    state.positions = vec![[2.0, 1.0], [4.5, 2.5]];
    world.set_state(&state).unwrap();
    assert_eq!(world.agents_cells(), vec![pos(1, 2), pos(2, 4)]);
}

#[test]
fn test_n_colours_single_agent() {
    let world = ContinuousWorld::try_from("S0 X").unwrap();
    assert_eq!(world.n_colours(), 1);
    assert_eq!(world.state_size(), 6 + 1);
}

/// Every agent has several possible starts.
const RANDOM_STARTS: &str = r#"
world_string = """
. . . . X
. . . . X
"""
[[agents]]
start_positions = [{i=0,j=0},{i=0,j=1},{i=0,j=2},{i=0,j=3}]
[[agents]]
start_positions = [{i=1,j=0},{i=1,j=1},{i=1,j=2},{i=1,j=3}]
"#;

fn draw_starts(world: &mut ContinuousWorld, n: usize) -> Vec<Vec<[f64; 2]>> {
    (0..n)
        .map(|_| {
            world.reset();
            world.agents_positions()
        })
        .collect()
}

#[test]
fn test_clone_copies_the_rng() {
    let mut world =
        ContinuousWorld::from_spec(MapSpec::parse(RANDOM_STARTS).unwrap(), Default::default())
            .unwrap();
    world.seed(3);
    draw_starts(&mut world, 5);
    let mut copy = world.clone();
    let expected = draw_starts(&mut world, 20);
    assert_eq!(draw_starts(&mut copy, 20), expected);
    // The draws are not all the same, so the test is meaningful.
    assert!(expected.iter().any(|starts| *starts != expected[0]));
}

#[test]
fn test_rng_state_round_trip() {
    let spec = || MapSpec::parse(RANDOM_STARTS).unwrap();
    let mut world = ContinuousWorld::from_spec(spec(), Default::default()).unwrap();
    world.seed(11);
    draw_starts(&mut world, 3);
    let rng_state = world.rng_state();
    let expected = draw_starts(&mut world, 10);

    let mut other = ContinuousWorld::from_spec(spec(), Default::default()).unwrap();
    other.set_rng_state(&rng_state);
    assert_eq!(draw_starts(&mut other, 10), expected);
}

#[test]
fn test_from_spec_uses_the_params() {
    let params = PhysicsParams {
        agent_radius: 0.25,
        ..Default::default()
    };
    let world = ContinuousWorld::from_spec(MapSpec::parse(MAP).unwrap(), params).unwrap();
    assert_eq!(world.params(), params);
}
