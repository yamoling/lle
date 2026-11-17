use crate::{
    World, WorldEvent,
    continuous::{ActionModel, ContinuousError, ContinuousWorld, PhysicsParams},
    shared::map::parse_v1,
};

const R: f64 = 0.3;
const STILL: [f64; 2] = [0.0, 0.0];

fn world(map: &str) -> ContinuousWorld {
    let mut world = ContinuousWorld::try_from(map).unwrap();
    world.reset();
    world
}

fn world_with(map: &str, params: PhysicsParams) -> ContinuousWorld {
    let mut world =
        ContinuousWorld::from_map(&parse_v1(map).unwrap().validate().unwrap(), params).unwrap();
    world.reset();
    world
}

fn run(world: &mut ContinuousWorld, actions: &[[f64; 2]], n_ticks: usize) -> Vec<WorldEvent> {
    (0..n_ticks)
        .flat_map(|_| world.tick(actions).unwrap())
        .collect()
}

fn assert_close(actual: [f64; 2], expected: [f64; 2], tol: f64) {
    let ok = (actual[0] - expected[0]).abs() <= tol && (actual[1] - expected[1]).abs() <= tol;
    assert!(ok, "expected {expected:?} (± {tol}), got {actual:?}");
}

fn distance(a: [f64; 2], b: [f64; 2]) -> f64 {
    ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2)).sqrt()
}

#[test]
fn test_builtin_levels_load() {
    for level in 1..=6 {
        let continuous = ContinuousWorld::get_level(level).unwrap();
        let discrete = World::get_level(level).unwrap();
        assert_eq!(continuous.n_agents(), discrete.n_agents(), "level {level}");
        assert_eq!(continuous.width(), discrete.width(), "level {level}");
        assert_eq!(continuous.height(), discrete.height(), "level {level}");
    }
}

#[test]
fn test_load_from_file() {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/resources/levels/lvl6");
    assert_eq!(ContinuousWorld::from_file(path).unwrap().n_agents(), 4);
}

#[test]
fn test_unsupported_features_are_all_listed() {
    let cases: [(&str, &[&str]); 4] = [
        ("S0 # X", &["boxes"]),
        ("S0 H0 P0 # X", &["boxes", "levers", "pressure_plates"]),
        ("S0 X\n;\n. .", &["layers"]),
        (
            "S0 TU0 B0 S1 X\n;\nX . . . .",
            &["buttons", "layers", "lifts"],
        ),
    ];
    for (map, expected) in cases {
        match ContinuousWorld::try_from(map) {
            Err(ContinuousError::UnsupportedFeature { mut features }) => {
                features.sort();
                assert_eq!(features, expected, "{map:?}");
            }
            other => panic!("{map:?}: expected UnsupportedFeature, got {other:?}"),
        }
    }
}

#[test]
fn test_reset_places_agents_at_start_cell_centres() {
    // (x, y) = (column + 0.5, row + 0.5).
    let mut world = world(
        "
        S0 . X
        .  . S1
        X  . .
        ",
    );
    let state = world.get_state();
    assert_eq!(world.agents_positions(), vec![[0.5, 0.5], [2.5, 1.5]]);
    assert_eq!(state.agents_alive, vec![true, true]);
    assert_eq!(state.agents_arrived, vec![false, false]);
    assert_eq!(state.velocities, vec![[0.0, 0.0], [0.0, 0.0]]);
    assert_eq!(state.tick, 0);

    run(&mut world, &[[1.0, 1.0], [-1.0, 0.0]], 5);
    world.reset();
    assert_eq!(world.get_state(), state);
}

#[test]
fn test_random_starts_are_seeded_cell_centres() {
    let mut world = world(
        r#"
world_string = """
. . . X
. . . X
"""
[[agents]]
start_positions = [{ i_min = 0, i_max = 1, j_min = 0, j_max = 2 }]
[[agents]]
start_positions = [{ i_min = 0, i_max = 1, j_min = 0, j_max = 2 }]
"#,
    );
    let mut seen = Vec::new();
    for seed in 0..20 {
        world.seed(seed);
        world.reset();
        let positions = world.agents_positions();
        world.seed(seed);
        world.reset();
        assert_eq!(world.agents_positions(), positions, "seed {seed}");
        assert_ne!(positions[0], positions[1]);
        for [x, y] in &positions {
            assert!([0.5, 1.5, 2.5].contains(x) && [0.5, 1.5].contains(y));
        }
        if !seen.contains(&positions) {
            seen.push(positions);
        }
    }
    assert!(seen.len() > 1, "random starts never vary with the seed");
}

#[test]
fn test_tick_rejects_wrong_number_of_actions() {
    let mut world = world("S0 . X\nS1 . X");
    let state = world.get_state();
    assert!(matches!(
        world.tick(&[STILL]),
        Err(ContinuousError::InvalidNumberOfActions {
            given: 1,
            expected: 2
        })
    ));
    assert_eq!(world.get_state(), state);
}

#[test]
fn test_tick_rejects_non_finite_actions() {
    let mut world = world("S0 . X\nS1 . X");
    let state = world.get_state();
    for bad in [
        [f64::NAN, 0.0],
        [0.0, f64::INFINITY],
        [f64::NEG_INFINITY, 0.0],
    ] {
        assert!(matches!(
            world.tick(&[STILL, bad]),
            Err(ContinuousError::InvalidAction { agent_id: 1, .. })
        ));
        assert_eq!(world.get_state(), state);
    }
}

#[test]
fn test_velocity_model_clips_actions_by_norm() {
    // Defaults: max_speed * dt = 0.1 cell per tick.
    let diagonal = 0.1 / 2f64.sqrt();
    let cases = [
        ([1.0, 0.0], [0.1, 0.0]),
        ([5.0, 0.0], [0.1, 0.0]),
        ([0.0, -0.5], [0.0, -0.05]),
        ([1.0, 1.0], [diagonal, diagonal]),
        ([-3.0, 3.0], [-diagonal, diagonal]),
    ];
    let mut world = world(
        "
        . . .  . X
        . . .  . .
        . . S0 . .
        . . .  . .
        . . .  . .
        ",
    );
    for (action, [dx, dy]) in cases {
        world.reset();
        world.tick(&[action]).unwrap();
        assert_close(world.agents_positions()[0], [2.5 + dx, 2.5 + dy], 1e-12);
    }
}

#[test]
fn test_acceleration_model() {
    let params = PhysicsParams {
        action_model: ActionModel::Acceleration,
        ..Default::default()
    };
    let mut world = world_with("S0 . . . . . . . . . . X", params);
    // v = damping * (0 + a * max_acceleration * dt) = 0.9 * 0.4.
    world.tick(&[[1.0, 0.0]]).unwrap();
    assert_close(world.get_state().velocities[0], [0.36, 0.0], 1e-12);
    assert_close(world.agents_positions()[0], [0.5 + 0.36 * 0.05, 0.5], 1e-12);
    // The speed saturates at max_speed after 8 ticks.
    run(&mut world, &[[1.0, 0.0]], 9);
    assert_close(world.get_state().velocities[0], [2.0, 0.0], 1e-9);
}

#[test]
fn test_touching_a_void_is_safe() {
    let params = PhysicsParams {
        agent_radius: 0.25,
        ..Default::default()
    };
    let mut world = world_with("S0 . V\nX  . .", params);
    let mut state = world.get_state();
    // Exactly r away from the void cell [2, 3] x [0, 1].
    state.positions[0] = [1.75, 0.5];
    world.set_state(&state).unwrap();
    assert!(run(&mut world, &[STILL], 3).is_empty());
    assert!(world.get_state().agents_alive[0]);
}

#[test]
fn test_overlapping_a_void_kills() {
    let params = PhysicsParams {
        agent_radius: 0.25,
        ..Default::default()
    };
    let mut world = world_with("S0 . V\nX  . .", params);
    let mut state = world.get_state();
    state.positions[0] = [1.75 + 1e-6, 0.5];
    world.set_state(&state).unwrap();
    assert_eq!(
        world.tick(&[STILL]).unwrap(),
        vec![WorldEvent::AgentDied { agent_id: 0 }]
    );
    assert!(!world.get_state().agents_alive[0]);
    // Dead agents no longer move nor emit events.
    let position = world.agents_positions()[0];
    assert!(run(&mut world, &[[-1.0, 0.0]], 5).is_empty());
    assert_eq!(world.agents_positions()[0], position);
}

#[test]
fn test_void_overlap_uses_the_distance_to_the_square() {
    // Void cell [1, 2] x [1, 2]. At (0.75, 0.75), the disc's bounding box overlaps the void
    // but the disc itself is 0.354 away from the corner (1, 1).
    let mut world = world(
        "
        S0 . .
        .  V .
        X  . .
        ",
    );
    let mut state = world.get_state();
    state.positions[0] = [0.75, 0.75];
    world.set_state(&state).unwrap();
    assert!(world.tick(&[STILL]).unwrap().is_empty());

    state.positions[0] = [0.8, 0.8];
    world.set_state(&state).unwrap();
    assert_eq!(
        world.tick(&[STILL]).unwrap(),
        vec![WorldEvent::AgentDied { agent_id: 0 }]
    );
}

#[test]
fn test_gem_is_collected_on_disc_overlap() {
    // 0.07 cell per tick. The pickup distance is r + gem_radius = 0.55, i.e. x > 0.95.
    let mut world = world("S0 G X");
    assert!(run(&mut world, &[[0.7, 0.0]], 6).is_empty()); // x = 0.92
    assert!(!world.get_state().gems_collected[0]);
    assert_eq!(
        world.tick(&[[0.7, 0.0]]).unwrap(), // x = 0.99
        vec![WorldEvent::GemCollected { agent_id: 0 }]
    );
    assert!(world.get_state().gems_collected[0]);
    // A collected gem is not collected again.
    assert!(run(&mut world, &[[0.1, 0.0]], 3).is_empty());
}

#[test]
fn test_gem_tie_goes_to_the_lowest_id() {
    // Both agents reach the gem at (1.5, 0.5) in tick 7, agent 1 being the closest one.
    let mut world = world("S0 G S1\nX  . X");
    let actions = [[0.7, 0.0], [-0.74, 0.0]];
    assert!(run(&mut world, &actions, 6).is_empty());
    assert_eq!(
        world.tick(&actions).unwrap(),
        vec![WorldEvent::GemCollected { agent_id: 0 }]
    );
}

#[test]
fn test_exit_requires_the_centre_in_the_exit_cell() {
    let mut world = world("S0 X .");
    // After 7 ticks, x = 0.99: the disc overlaps the exit cell, but not the centre.
    assert!(run(&mut world, &[[0.7, 0.0]], 7).is_empty());
    assert!(!world.get_state().agents_arrived[0]);
    assert_eq!(
        world.tick(&[[0.7, 0.0]]).unwrap(), // x = 1.06
        vec![WorldEvent::AgentExit { agent_id: 0 }]
    );
    assert!(world.get_state().agents_arrived[0]);
}

#[test]
fn test_arrived_agent_is_frozen_but_still_blocks() {
    // Agent 0 arrives at x = 3.94, then agent 1 runs into it from behind.
    let mut world = world("X . . X S0 . S1");
    assert_eq!(
        run(&mut world, &[[-0.7, 0.0], STILL], 8),
        vec![WorldEvent::AgentExit { agent_id: 0 }]
    );
    let arrived_at = world.agents_positions()[0];
    assert!(run(&mut world, &[[-1.0, 0.0], [-1.0, 0.0]], 40).is_empty());
    let state = world.get_state();
    assert_eq!(state.positions[0], arrived_at);
    assert_eq!(state.velocities[0], [0.0, 0.0]);
    assert!((distance(state.positions[0], state.positions[1]) - 2.0 * R).abs() < 1e-6);
    assert!(!state.agents_arrived[1]);
}

#[test]
fn test_gem_is_collected_in_the_tick_its_collector_dies() {
    // Agent 1 walks north into the L0 beam (y = 0.5). With gem_radius = 0.05, the gem pickup
    // and the beam overlap both start at y < 0.85, in tick 7. Gems resolve before lasers.
    let params = PhysicsParams {
        gem_radius: 0.05,
        ..Default::default()
    };
    let mut world = world_with(
        "
        L0E G  .
        S0  S1 X
        .   .  X
        ",
        params,
    );
    assert!(run(&mut world, &[STILL, [0.0, -1.0]], 6).is_empty());
    assert_eq!(
        world.tick(&[STILL, [0.0, -1.0]]).unwrap(),
        vec![
            WorldEvent::GemCollected { agent_id: 1 },
            WorldEvent::AgentDied { agent_id: 1 },
        ]
    );
}

#[test]
fn test_step_runs_n_ticks() {
    let mut world = world("S0 . . . . X");
    let outcome = world.step(&[[1.0, 0.0]], 3).unwrap();
    assert_eq!(outcome.n_ticks, 3);
    assert!(outcome.events.is_empty());
    assert_eq!(world.get_state().tick, 3);
    assert_close(world.agents_positions()[0], [0.8, 0.5], 1e-12);
}

#[test]
fn test_step_tags_events_with_their_tick_index() {
    // Gem at tick index 6 (x = 0.99), exit at tick index 21 (x = 2.04), counted from the
    // start of the step, not from the start of the episode.
    let mut world = world("S0 G X");
    run(&mut world, &[STILL], 3);
    let outcome = world.step(&[[0.7, 0.0]], 50).unwrap();
    assert_eq!(
        outcome.events,
        vec![
            (6, WorldEvent::GemCollected { agent_id: 0 }),
            (21, WorldEvent::AgentExit { agent_id: 0 }),
        ]
    );
    // Every alive agent has arrived: early stop.
    assert_eq!(outcome.n_ticks, 22);
    assert_eq!(world.get_state().tick, 25);
}

#[test]
fn test_step_does_not_stop_while_an_agent_has_not_arrived() {
    let mut world = world("S0 X .\nS1 . X");
    let outcome = world.step(&[[0.7, 0.0], STILL], 20).unwrap();
    assert_eq!(
        outcome.events,
        vec![(7, WorldEvent::AgentExit { agent_id: 0 })]
    );
    assert_eq!(outcome.n_ticks, 20);
}

#[test]
fn test_step_stops_on_death() {
    // The void cell starts at x = 2: the agent dies when x > 1.7, at tick index 17 (x = 1.76).
    let mut world = world("S0 . V\nS1 . X\nX  . .");
    let outcome = world.step(&[[0.7, 0.0], STILL], 50).unwrap();
    assert_eq!(
        outcome.events,
        vec![(17, WorldEvent::AgentDied { agent_id: 0 })]
    );
    assert_eq!(outcome.n_ticks, 18);
    assert_eq!(world.get_state().tick, 18);
}

#[test]
fn test_random_starts_exclude_lethal_beams() {
    // Row 0 is on the unblocked L0 beam, lethal to agent 1 (colour 1). As in LLE, these cells
    // are not possible starts.
    let mut world = world(
        r#"
world_string = """
L0E . . .
S0  . . X
.   . . X
"""
[[agents]]
[[agents]]
start_positions = [{ i_min = 0, i_max = 1, j_min = 1, j_max = 3 }]
"#,
    );
    for seed in 0..20 {
        world.seed(seed);
        world.reset();
        assert!(
            world.tick(&[STILL, STILL]).unwrap().is_empty(),
            "seed {seed}"
        );
    }
}
