use crate::{
    continuous::{
        ContinuousError, ContinuousWorld, ContinuousWorldState, LaserSourceInfo, PhysicsParams,
    },
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

// ----- Static features -----

#[test]
fn test_static_features() {
    let world = world();
    assert_eq!(world.static_features_size(), 2 * 2 + 2 + 6);
    #[rustfmt::skip]
    let expected = vec![
        // The exits (0, 4) and (1, 4).
        4.5 / WIDTH, 0.5 / HEIGHT,
        4.5 / WIDTH, 1.5 / HEIGHT,
        // The gem (0, 2).
        2.5 / WIDTH, 0.5 / HEIGHT,
        // The source (2, 1), pointing north.
        1.5 / WIDTH, 2.5 / HEIGHT, 1.0, 0.0, 0.0, 0.0,
    ];
    assert_eq!(world.static_features(), expected);
}

// ----- Layered -----

/// Agents of colours 0 and 1, a beam of colour 1 over a void, a wall, a gem and two exits.
///
/// ```text
/// S0  . G . X
/// L1E . . V X
/// S1  . @ . .
/// ```
const LAYERED_MAP: &str = "S0 . G . X\nL1E . . V X\nS1 . @ . .";
const A0: usize = 0;
const A1: usize = 1;
const LASER_0: usize = 2;
const LASER_1: usize = 3;
const WALL: usize = 4;
const VOID: usize = 5;
const GEM: usize = 6;
const EXIT: usize = 7;

/// The channel `channel` of the layered observation of `world`, as rows.
fn channel(world: &ContinuousWorld, res: usize, channel: usize) -> Vec<Vec<f32>> {
    let [_, height, width] = world.layered_shape(res).unwrap();
    let layered = world.layered(res).unwrap();
    let start = channel * height * width;
    layered[start..start + height * width]
        .chunks(width)
        .map(<[f32]>::to_vec)
        .collect()
}

/// The cells of `rows` holding `value`.
fn cells_with(rows: &[Vec<f32>], value: f32) -> Vec<(usize, usize)> {
    let mut res = vec![];
    for (i, row) in rows.iter().enumerate() {
        for (j, &v) in row.iter().enumerate() {
            if v == value {
                res.push((i, j));
            }
        }
    }
    res
}

fn layered_world() -> ContinuousWorld {
    ContinuousWorld::try_from(LAYERED_MAP).unwrap()
}

#[test]
fn test_layered_shape() {
    let world = layered_world();
    assert_eq!(world.layered_shape(1).unwrap(), [8, 3, 5]);
    assert_eq!(world.layered_shape(4).unwrap(), [8, 12, 20]);
    assert_eq!(world.layered(3).unwrap().len(), 8 * 9 * 15);
    assert!(matches!(
        world.layered_shape(0),
        Err(ContinuousError::InvalidParams { .. })
    ));
    assert!(matches!(
        world.layered(0),
        Err(ContinuousError::InvalidParams { .. })
    ));
}

#[test]
fn test_layered_at_reset_with_res_1() {
    let world = layered_world();
    let only = |channel_id: usize, cells: Vec<(usize, usize)>| {
        let rows = channel(&world, 1, channel_id);
        assert_eq!(cells_with(&rows, 1.0), cells, "channel {channel_id}");
        let non_zero = rows.iter().flatten().filter(|&&v| v != 0.0).count();
        let minus_one = cells_with(&rows, -1.0).len();
        assert_eq!(non_zero, cells.len() + minus_one, "channel {channel_id}");
    };
    only(A0, vec![(0, 0)]);
    only(A1, vec![(2, 0)]);
    only(LASER_0, vec![]);
    // The beam runs over the void and the exit, and its source is -1.
    only(LASER_1, vec![(1, 1), (1, 2), (1, 3), (1, 4)]);
    assert_eq!(cells_with(&channel(&world, 1, LASER_1), -1.0), vec![(1, 0)]);
    // The sources are walls, as in LLE.
    only(WALL, vec![(1, 0), (2, 2)]);
    only(VOID, vec![(1, 3)]);
    only(GEM, vec![(0, 2)]);
    only(EXIT, vec![(0, 4), (1, 4)]);
}

#[test]
fn test_layered_beam_is_cut_at_the_disc_of_the_blocker() {
    let mut world = layered_world();
    let mut state = world.get_state();
    // Agent 1 (colour 1) stands in the beam, at the centre of the cell (1, 2): the beam stops at
    // its disc, at x = 2.2, so it reaches into the cell (1, 2), unlike in LLE, which turns the
    // beam off from the cell of the blocker on.
    state.positions[1] = [2.5, 1.5];
    world.set_state(&state).unwrap();
    let laser = channel(&world, 1, LASER_1);
    assert_eq!(cells_with(&laser, 1.0), vec![(1, 1), (1, 2)]);
    assert_eq!(cells_with(&channel(&world, 1, A1), 1.0), vec![(1, 2)]);
    // At res 4, the pixel spanning [2, 2.25] overlaps the beam, the next one does not.
    let laser = channel(&world, 4, LASER_1);
    let expected: Vec<_> = [5, 6]
        .into_iter()
        .flat_map(|row| (4..=8).map(move |col| (row, col)))
        .collect();
    assert_eq!(cells_with(&laser, 1.0), expected);
    // A disabled source keeps its -1 but has no beam.
    state.sources_enabled[0] = false;
    world.set_state(&state).unwrap();
    let laser = channel(&world, 1, LASER_1);
    assert_eq!(cells_with(&laser, 1.0), vec![]);
    assert_eq!(cells_with(&laser, -1.0), vec![(1, 0)]);
}

#[test]
fn test_layered_beam_overlaps_pixels_with_a_positive_area() {
    let world = layered_world();
    // The beam spans x in [1, 5] and y in [1.45, 1.55].
    let beam = |res: usize, rows: Vec<usize>, cols: std::ops::Range<usize>| {
        let expected: Vec<_> = rows
            .into_iter()
            .flat_map(|row| cols.clone().map(move |col| (row, col)))
            .collect();
        assert_eq!(
            cells_with(&channel(&world, res, LASER_1), 1.0),
            expected,
            "res {res}"
        );
    };
    // res 1: the cells of the beam.
    beam(1, vec![1], 1..5);
    // res 3: the middle row of the cells, [4/3, 5/3]; the pixels that only touch x = 1 or
    // x = 5 are excluded.
    beam(3, vec![4], 3..15);
    // res 4: the axis y = 1.5 is a pixel edge, so the two rows on either side are lit.
    beam(4, vec![5, 6], 4..20);
    // A wider beam covers more rows.
    let params = PhysicsParams {
        beam_width: 0.6,
        ..PhysicsParams::default()
    };
    let wide = ContinuousWorld::from_spec(MapSpec::parse(LAYERED_MAP).unwrap(), params).unwrap();
    assert_eq!(
        cells_with(&channel(&wide, 4, LASER_1), 1.0).len(),
        4 * 16,
        "y in [1.2, 1.8] overlaps the rows 4 to 7"
    );
}

#[test]
fn test_layered_hides_dead_agents_and_collected_gems() {
    let mut world = layered_world();
    let mut state = world.get_state();
    state.agents_alive[0] = false;
    state.agents_arrived[1] = true;
    state.gems_collected[0] = true;
    world.set_state(&state).unwrap();
    assert_eq!(cells_with(&channel(&world, 1, A0), 1.0), vec![]);
    assert_eq!(cells_with(&channel(&world, 1, A1), 1.0), vec![(2, 0)]);
    assert_eq!(cells_with(&channel(&world, 1, GEM), 1.0), vec![]);
}

#[test]
fn test_layered_with_res_4() {
    let world = layered_world();
    // A disc of radius 0.3 at a cell centre covers the 4 central pixels of the cell (their
    // centres are 0.177 away), and so does the pickup circle of radius 0.25.
    let centre_square = |i: usize, j: usize| {
        vec![
            (4 * i + 1, 4 * j + 1),
            (4 * i + 1, 4 * j + 2),
            (4 * i + 2, 4 * j + 1),
            (4 * i + 2, 4 * j + 2),
        ]
    };
    assert_eq!(
        cells_with(&channel(&world, 4, A0), 1.0),
        centre_square(0, 0)
    );
    assert_eq!(
        cells_with(&channel(&world, 4, A1), 1.0),
        centre_square(2, 0)
    );
    assert_eq!(
        cells_with(&channel(&world, 4, GEM), 1.0),
        centre_square(0, 2)
    );
    // The source cell is -1 (the beam itself is checked above).
    assert_eq!(cells_with(&channel(&world, 4, LASER_1), -1.0).len(), 16);
    // Cells are filled entirely.
    assert_eq!(cells_with(&channel(&world, 4, WALL), 1.0).len(), 2 * 16);
    assert_eq!(cells_with(&channel(&world, 4, EXIT), 1.0).len(), 2 * 16);
}

#[test]
fn test_layered_disc_between_cells() {
    let mut world = layered_world();
    let mut state = world.get_state();
    // Agent 0 between the cells (0, 0) and (0, 1): its disc covers the pixel centres within
    // 0.3 of (1.0, 0.5) at res 4: x in {0.875, 1.125} and y in {0.375, 0.625}.
    state.positions[0] = [1.0, 0.5];
    world.set_state(&state).unwrap();
    assert_eq!(
        cells_with(&channel(&world, 4, A0), 1.0),
        vec![(1, 3), (1, 4), (2, 3), (2, 4)]
    );
}

// ----- Lidar -----

/// The lidar value of `category` for the ray `ray`, out of `n_rays` rays of length `range`.
fn lidar_value(
    world: &ContinuousWorld,
    agent: usize,
    n_rays: usize,
    range: f64,
    ray: usize,
    category: usize,
) -> f32 {
    let lidar = world.lidar(agent, n_rays, range).unwrap().unwrap();
    lidar[ray * world.lidar_categories() + category]
}

fn assert_lidar(actual: f32, expected: f64) {
    assert!(
        (actual as f64 - expected).abs() < 1e-6,
        "expected {expected}, got {actual}"
    );
}

/// Along the row of agent 0: a void, a gem, an exit, agent 1, a beam of colour 1 (going north
/// from its source below) and a wall.
///
/// ```text
/// S0 . V G X S1 .   @
/// X  . . . . .  L1N .
/// ```
const LIDAR_ROW: &str = "S0 . V G X S1 . @\nX . . . . . L1N .";

#[test]
fn test_lidar_size_and_layout() {
    let world = ContinuousWorld::try_from(LIDAR_ROW).unwrap();
    assert_eq!(world.n_colours(), 2);
    assert_eq!(world.lidar_categories(), 7);
    assert_eq!(world.lidar_size(16).unwrap(), 16 * 7 + 8);
    assert_eq!(world.lidar_size(1).unwrap(), 7 + 8);
    let lidar = world.lidar(0, 16, 5.0).unwrap().unwrap();
    assert_eq!(lidar.len(), world.lidar_size(16).unwrap());
    // The own block of the observer comes last, in absolute coordinates.
    let observation = world.observation(0).unwrap();
    assert_eq!(&lidar[16 * 7..], &observation[..8]);
    assert!(lidar.iter().all(|v| (0.0..=1.0).contains(v)));
}

#[test]
fn test_lidar_axis_aligned_rays() {
    let world = ContinuousWorld::try_from(LIDAR_ROW).unwrap();
    let range = 10.0;
    let east = |category| lidar_value(&world, 0, 8, range, 0, category);
    // The wall (0, 7) starts at x = 7, 6.5 away; the source (1, 6) is not on the ray.
    assert_lidar(east(0), 6.5 / range);
    assert_lidar(east(1), 1.5 / range);
    // Agent 1, of radius 0.3, centred at x = 5.5.
    assert_lidar(east(2), 4.7 / range);
    // The gem, of radius 0.25, centred at x = 3.5.
    assert_lidar(east(3), 2.75 / range);
    assert_lidar(east(4), 3.5 / range);
    assert_lidar(east(5), 1.0);
    // The beam of colour 1, 0.1 wide, along x = 6.5.
    assert_lidar(east(6), 5.95 / range);
    // Ray 2 points north (nothing, the boundary is not a wall), ray 4 west and ray 6 south.
    for category in 0..7 {
        assert_lidar(lidar_value(&world, 0, 8, range, 2, category), 1.0);
        assert_lidar(lidar_value(&world, 0, 8, range, 4, category), 1.0);
    }
    assert_lidar(lidar_value(&world, 0, 8, range, 6, 4), 0.5 / range);
    // With a shorter range, far hits are out of range.
    assert_lidar(lidar_value(&world, 0, 8, 2.0, 0, 1), 0.75);
    assert_lidar(lidar_value(&world, 0, 8, 2.0, 0, 3), 1.0);
}

#[test]
fn test_lidar_diagonal_rays() {
    let sqrt2 = 2.0_f64.sqrt();
    // Along the south-east diagonal of agent 0: a wall, a gem and an exit.
    let world = ContinuousWorld::try_from("S0 . . .\n. @ . .\n. . G .\n. . . X").unwrap();
    // Ray 7 of 8 points south-east, ray 1 north-east.
    let south_east = |category| lidar_value(&world, 0, 8, 5.0, 7, category);
    assert_lidar(south_east(0), 0.5 * sqrt2 / 5.0);
    assert_lidar(south_east(3), (2.0 * sqrt2 - 0.25) / 5.0);
    assert_lidar(south_east(4), 2.5 * sqrt2 / 5.0);
    assert_lidar(lidar_value(&world, 0, 8, 5.0, 1, 0), 1.0);

    // Along the same diagonal: a void, agent 1 and a beam of colour 1 (going south along
    // x = 3.5).
    let world =
        ContinuousWorld::try_from("S0 . . L1S\n. V . .\n. . S1 .\n. . . X\nX . . .").unwrap();
    let south_east = |category| lidar_value(&world, 0, 8, 5.0, 7, category);
    assert_lidar(south_east(0), 1.0);
    assert_lidar(south_east(1), 0.5 * sqrt2 / 5.0);
    assert_lidar(south_east(2), (2.0 * sqrt2 - 0.3) / 5.0);
    assert_lidar(south_east(4), 2.5 * sqrt2 / 5.0);
    assert_lidar(south_east(5), 1.0);
    assert_lidar(south_east(6), 2.95 * sqrt2 / 5.0);
}

#[test]
fn test_lidar_rays_turn_counter_clockwise_on_screen() {
    // An exit to the north-east of agent 0 at (1.5, 1.5) and walls north and west.
    let world = ContinuousWorld::try_from(". @ X\n@ S0 .\n. . X").unwrap();
    let hit = |ray| lidar_value(&world, 0, 4, 5.0, ray, 0);
    // East: nothing; north: the wall (0, 1); west: the wall (1, 0); south: nothing.
    assert_lidar(hit(0), 1.0);
    assert_lidar(hit(1), 0.1);
    assert_lidar(hit(2), 0.1);
    assert_lidar(hit(3), 1.0);
    // Ray 1 of 8 points north-east, to the exit (0, 2).
    let exit = lidar_value(&world, 0, 8, 5.0, 1, 4);
    assert_lidar(exit, 0.5 * 2.0_f64.sqrt() / 5.0);
}

#[test]
fn test_lidar_beam_cut_and_dead_agents() {
    let mut world = ContinuousWorld::try_from(LIDAR_ROW).unwrap();
    let mut state = world.get_state();
    // Agent 1 (colour 1) stands in the beam, at (6.5, 0.5): the beam stops at its disc, at
    // y = 0.8, so the ray along y = 0.5 misses it.
    state.positions[1] = [6.5, 0.5];
    world.set_state(&state).unwrap();
    assert_lidar(lidar_value(&world, 0, 8, 10.0, 0, 6), 1.0);
    // Agent 1 is now hit at x = 6.5 - 0.3, 5.7 away.
    assert_lidar(lidar_value(&world, 0, 8, 10.0, 0, 2), 0.57);
    // Dead agents are not seen, and a dead observer still gets its lidar and its flags.
    state.agents_alive[1] = false;
    state.agents_alive[0] = false;
    world.set_state(&state).unwrap();
    assert_lidar(lidar_value(&world, 0, 8, 10.0, 0, 2), 1.0);
    let lidar = world.lidar(0, 8, 10.0).unwrap().unwrap();
    let block = &lidar[8 * 7..];
    assert_eq!(&block[..6], &[0.5 / 8.0, 0.5 / 2.0, 0.0, 0.0, 0.0, 0.0]);
}

#[test]
fn test_lidar_errors() {
    let world = ContinuousWorld::try_from(LIDAR_ROW).unwrap();
    for (n_rays, range) in [
        (0, 5.0),
        (8, 0.0),
        (8, -1.0),
        (8, f64::NAN),
        (8, f64::INFINITY),
    ] {
        assert!(matches!(
            world.lidar(0, n_rays, range),
            Err(ContinuousError::InvalidParams { .. })
        ));
    }
    assert!(matches!(
        world.lidar_size(0),
        Err(ContinuousError::InvalidParams { .. })
    ));
    assert_eq!(world.lidar(2, 8, 5.0).unwrap(), None);
}
