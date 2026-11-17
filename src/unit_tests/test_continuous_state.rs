use rand::{RngExt, SeedableRng, rngs::StdRng};

use crate::{
    WorldEvent,
    continuous::{
        ActionModel, ContinuousError, ContinuousWorld, ContinuousWorldState, PhysicsParams,
    },
    shared::map::parse_v1,
};

const LEVEL_6: &str = include_str!("../../resources/levels/lvl6");

fn level_6(action_model: ActionModel) -> ContinuousWorld {
    let params = PhysicsParams {
        action_model,
        ..Default::default()
    };
    let mut world =
        ContinuousWorld::from_map(&parse_v1(LEVEL_6).unwrap().validate().unwrap(), params).unwrap();
    world.reset();
    world
}

fn random_actions(rng: &mut StdRng, n_agents: usize) -> Vec<[f64; 2]> {
    (0..n_agents)
        .map(|_| [rng.random_range(-1.0..=1.0), rng.random_range(-1.0..=1.0)])
        .collect()
}

/// Runs the given actions and records the state and events after every tick.
fn replay(
    world: &mut ContinuousWorld,
    actions: &[Vec<[f64; 2]>],
) -> Vec<(ContinuousWorldState, Vec<WorldEvent>)> {
    actions
        .iter()
        .map(|a| {
            let events = world.tick(a).unwrap();
            (world.get_state(), events)
        })
        .collect()
}

#[test]
fn test_restore_and_replay_is_bit_identical() {
    for action_model in [ActionModel::Velocity, ActionModel::Acceleration] {
        let mut world = level_6(action_model);
        let mut rng = StdRng::seed_from_u64(1);
        for _ in 0..30 {
            world.tick(&random_actions(&mut rng, 4)).unwrap();
        }
        let snapshot = world.get_state();
        let actions: Vec<_> = (0..300).map(|_| random_actions(&mut rng, 4)).collect();
        let first = replay(&mut world, &actions);

        world.set_state(&snapshot).unwrap();
        assert_eq!(world.get_state(), snapshot);
        assert_eq!(replay(&mut world, &actions), first, "{action_model:?}");

        // A fresh world has no hidden state that the snapshot misses.
        let mut fresh = level_6(action_model);
        fresh.set_state(&snapshot).unwrap();
        assert_eq!(replay(&mut fresh, &actions), first, "{action_model:?}");
    }
}

#[test]
fn test_state_round_trip_with_a_dead_agent_under_an_alive_one() {
    // Agent 1 walks north into the L0 beam and dies at (2.5, 0.8), at its 7th tick. Agent 0
    // then walks onto that very spot: dead agents do not collide, and such a state must be
    // restorable.
    let mut world = ContinuousWorld::try_from(
        "
        L0E .  .  .
        .   S0 S1 X
        .   .  .  X
        ",
    )
    .unwrap();
    world.reset();
    for _ in 0..7 {
        world.tick(&[[0.0, 0.0], [0.0, -1.0]]).unwrap();
    }
    let dead_at = world.agents_positions()[1];
    for _ in 0..10 {
        world.tick(&[[1.0, 0.0], [0.0, 0.0]]).unwrap();
    }
    for _ in 0..7 {
        world.tick(&[[0.0, -1.0], [0.0, 0.0]]).unwrap();
    }
    let state = world.get_state();
    assert_eq!(state.agents_alive, vec![true, false]);
    let [x, y] = state.positions[0];
    assert!(((x - dead_at[0]).powi(2) + (y - dead_at[1]).powi(2)).sqrt() < 2.0 * 0.3 - 0.1);

    world.reset();
    world.set_state(&state).unwrap();
    assert_eq!(world.get_state(), state);
}

#[test]
fn test_set_state_rejects_overlapping_agents() {
    // r = 0.3125 so that the contact distance 2r = 0.625 is exact.
    let params = PhysicsParams {
        agent_radius: 0.3125,
        ..Default::default()
    };
    let mut world = ContinuousWorld::from_map(
        &parse_v1("S0 . . X\nS1 . . X").unwrap().validate().unwrap(),
        params,
    )
    .unwrap();
    world.reset();
    let valid = world.get_state();
    let mut state = valid.clone();
    state.positions = vec![[1.5, 0.5], [1.5, 1.05]];
    assert!(matches!(
        world.set_state(&state),
        Err(ContinuousError::InvalidState { .. })
    ));
    assert_eq!(world.get_state(), valid);

    state.positions = vec![[1.5, 0.5], [1.5, 1.125]];
    assert!(world.set_state(&state).is_ok());
}

#[test]
fn test_set_state_rejects_agents_in_walls_or_outside_the_grid() {
    let mut world = ContinuousWorld::try_from("S0 @ . X\nS1 . . X").unwrap();
    world.reset();
    for position in [[1.2, 0.5], [0.1, 0.5], [3.5, 1.9], [-1.0, 0.5]] {
        let mut state = world.get_state();
        state.positions[0] = position;
        assert!(
            matches!(
                world.set_state(&state),
                Err(ContinuousError::InvalidState { .. })
            ),
            "{position:?}"
        );
    }
}

#[test]
fn test_set_state_rejects_wrong_lengths() {
    let mut world = ContinuousWorld::try_from("S0 G . X\nS1 . . X").unwrap();
    world.reset();
    let valid = world.get_state();

    let mut invalid = vec![valid.clone(); 6];
    invalid[0].positions.pop();
    invalid[1].velocities.push([0.0, 0.0]);
    invalid[2].agents_alive.pop();
    invalid[3].agents_arrived.push(false);
    invalid[4].gems_collected.clear();
    invalid[5].sources_enabled.push(true);
    for state in &invalid {
        assert!(world.set_state(state).is_err(), "{state:?}");
    }
    assert_eq!(world.get_state(), valid);
}

#[test]
fn test_set_state_accepts_every_state_of_a_crowded_rollout() {
    // Under the acceleration model, agent-pair corrections must not leave a velocity that
    // set_state then rejects.
    let params = PhysicsParams {
        action_model: ActionModel::Acceleration,
        ..Default::default()
    };
    let map = "S0 . . . . X\nS1 . . . . X\nS2 . . . . X\nS3 . . . . X\n. . . . . .\n. . . . . .";
    let mut world =
        ContinuousWorld::from_map(&parse_v1(map).unwrap().validate().unwrap(), params).unwrap();
    let mut rng = StdRng::seed_from_u64(148);
    let mut actions = vec![[0.0, 0.0]; 4];
    for t in 0..50 {
        for action in actions.iter_mut() {
            if rng.random_bool(0.2) {
                *action = [rng.random_range(-1.0..=1.0), rng.random_range(-1.0..=1.0)];
            }
        }
        world.tick(&actions).unwrap();
        let state = world.get_state();
        if let Err(e) = world.set_state(&state) {
            panic!("tick {t}: {e}");
        }
    }
}
