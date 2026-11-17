use rand::{RngExt, SeedableRng, rngs::StdRng, seq::SliceRandom};

use crate::continuous::{ActionModel, ContinuousWorld, PhysicsParams};

const R: f64 = 0.3;
const TOL: f64 = 1e-9;
const STILL: [f64; 2] = [0.0, 0.0];

fn world(map: &str) -> ContinuousWorld {
    let mut world = ContinuousWorld::try_from(map).unwrap();
    world.reset();
    world
}

fn distance(a: [f64; 2], b: [f64; 2]) -> f64 {
    ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2)).sqrt()
}

/// Distance from `p` to the unit square of cell `(i, j)`.
fn distance_to_cell(p: [f64; 2], (i, j): (usize, usize)) -> f64 {
    let (j, i) = (j as f64, i as f64);
    let dx = (j - p[0]).max(p[0] - (j + 1.0)).max(0.0);
    let dy = (i - p[1]).max(p[1] - (i + 1.0)).max(0.0);
    (dx * dx + dy * dy).sqrt()
}

/// No alive disc overlaps another one, a wall, or the grid boundary.
fn assert_no_overlap(world: &ContinuousWorld, walls: &[(usize, usize)]) {
    let state = world.get_state();
    let alive: Vec<[f64; 2]> = state
        .positions
        .iter()
        .zip(&state.agents_alive)
        .filter(|&(_, &alive)| alive)
        .map(|(&p, _)| p)
        .collect();
    let (width, height) = (world.width() as f64, world.height() as f64);
    for (a, &p) in alive.iter().enumerate() {
        assert!(
            p[0] >= R - TOL && p[0] <= width - R + TOL,
            "{p:?} out of bounds"
        );
        assert!(
            p[1] >= R - TOL && p[1] <= height - R + TOL,
            "{p:?} out of bounds"
        );
        for &wall in walls {
            assert!(
                distance_to_cell(p, wall) >= R - TOL,
                "{p:?} in wall {wall:?}"
            );
        }
        for &q in &alive[a + 1..] {
            assert!(distance(p, q) >= 2.0 * R - TOL, "{p:?} overlaps {q:?}");
        }
    }
}

fn random_direction(rng: &mut StdRng) -> [f64; 2] {
    let angle = rng.random_range(0.0..std::f64::consts::TAU);
    [angle.cos(), angle.sin()]
}

#[test]
fn test_agent_slides_along_a_wall() {
    // Driven south-east, the agent hits the wall row at y = 1 - r = 0.7, then slides east
    // at the tangential speed 0.1 / sqrt(2) per tick.
    let mut world = world(
        "
        S0 . . . . . X
        @  @ @ @ @ @ @
        ",
    );
    for _ in 0..5 {
        world.tick(&[[1.0, 1.0]]).unwrap();
    }
    for _ in 0..5 {
        let before = world.agents_positions()[0];
        world.tick(&[[1.0, 1.0]]).unwrap();
        let after = world.agents_positions()[0];
        assert!((after[1] - 0.7).abs() < TOL, "{after:?}");
        assert!((after[0] - before[0] - 0.1 / 2f64.sqrt()).abs() < TOL);
    }
}

#[test]
fn test_acceleration_model_loses_the_normal_velocity_against_a_wall() {
    let params = PhysicsParams {
        action_model: ActionModel::Acceleration,
        ..Default::default()
    };
    let config = crate::shared::map::parse_v1("S0 . . . . . X\n@ @ @ @ @ @ @").unwrap();
    let mut world = ContinuousWorld::from_map(&config.validate().unwrap(), params).unwrap();
    world.reset();
    for _ in 0..10 {
        world.tick(&[[1.0, 1.0]]).unwrap();
    }
    let [vx, vy] = world.get_state().velocities[0];
    assert!((world.agents_positions()[0][1] - 0.7).abs() < TOL);
    assert!(vy.abs() < TOL, "normal velocity {vy}");
    assert!(vx > 0.5, "tangential velocity {vx}");
}

#[test]
fn test_agent_driven_into_a_corner_stops() {
    let mut world = world(
        "
        @ @  @
        @ S0 .
        @ .  X
        ",
    );
    for _ in 0..20 {
        world.tick(&[[-1.0, -1.0]]).unwrap();
    }
    let corner = world.agents_positions()[0];
    assert!(distance(corner, [1.0 + R, 1.0 + R]) < TOL, "{corner:?}");
    world.tick(&[[-1.0, -1.0]]).unwrap();
    assert!(distance(world.agents_positions()[0], corner) < TOL);
}

#[test]
fn test_no_tunnelling_through_a_one_cell_wall() {
    let mut world = world(
        "
        .  . @ . .
        .  . @ . .
        S0 . @ . X
        .  . @ . .
        .  . @ . .
        ",
    );
    let walls: Vec<_> = (0..5).map(|i| (i, 2)).collect();
    let mut rng = StdRng::seed_from_u64(0);
    let mut action = random_direction(&mut rng);
    let mut max_x: f64 = 0.0;
    for _ in 0..10_000 {
        if rng.random_bool(0.1) {
            action = random_direction(&mut rng);
        }
        world.tick(&[action]).unwrap();
        assert_no_overlap(&world, &walls);
        let x = world.agents_positions()[0][0];
        assert!(x < 2.0, "the agent crossed the wall: x = {x}");
        max_x = max_x.max(x);
    }
    assert!(max_x > 2.0 - R - 1e-3, "the agent never reached the wall");
}

#[test]
fn test_head_on_agents_stop_in_contact() {
    let mut world = world("S0 . . S1\nX  . . X");
    let mut previous = world.agents_positions();
    for _ in 0..30 {
        world.tick(&[[1.0, 0.0], [-1.0, 0.0]]).unwrap();
        let current = world.agents_positions();
        assert!(
            current[0][0] >= previous[0][0] - TOL,
            "agent 0 moved backwards"
        );
        assert!(
            current[1][0] <= previous[1][0] + TOL,
            "agent 1 moved backwards"
        );
        previous = current;
    }
    let [p0, p1] = [previous[0], previous[1]];
    assert!((distance(p0, p1) - 2.0 * R).abs() < 1e-6, "{p0:?} {p1:?}");
    // Symmetric inputs give a symmetric result, whatever the agent ids.
    assert!((p0[0] + p1[0] - 4.0).abs() < TOL);
    assert_eq!(world.n_collision_fallbacks(), 0);
}

#[test]
fn test_moving_agent_does_not_displace_a_stationary_one() {
    let mut world = world("S0 . S1 .\nX  . X  .");
    let stationary = world.agents_positions()[1];
    for _ in 0..30 {
        world.tick(&[[1.0, 0.0], STILL]).unwrap();
        assert!(distance(world.agents_positions()[1], stationary) < 1e-12);
    }
    let mover = world.agents_positions()[0];
    assert!(
        (distance(mover, stationary) - 2.0 * R).abs() < 1e-6,
        "{mover:?}"
    );
    assert_eq!(world.n_collision_fallbacks(), 0);
}

#[test]
fn test_agents_cannot_swap_in_a_one_cell_corridor() {
    // Wiggling vertically does not help: the corridor leaves 0.4 of vertical play, less than 2r.
    let mut world = world(
        "
        @  @ @ @ @ @
        S0 . . . . S1
        @  @ @ @ @ @
        X  . . . . X
        ",
    );
    let walls: Vec<_> = (0..6).flat_map(|j| [(0, j), (2, j)]).collect();
    for t in 0..200 {
        let up = if (t / 5) % 2 == 0 { 1.0 } else { -1.0 };
        world.tick(&[[1.0, up], [-1.0, -up]]).unwrap();
        let [p0, p1] = [world.agents_positions()[0], world.agents_positions()[1]];
        assert!(p0[0] < p1[0], "agents swapped: {p0:?} {p1:?}");
        assert_no_overlap(&world, &walls);
    }
}

#[test]
fn test_no_overlap_under_random_actions_on_random_maps() {
    for seed in 0..20 {
        let mut rng = StdRng::seed_from_u64(seed);
        let (width, height) = (rng.random_range(4..=7), rng.random_range(4..=7));
        let n_agents = rng.random_range(2..=4);
        let mut cells: Vec<(usize, usize)> = (0..height)
            .flat_map(|i| (0..width).map(move |j| (i, j)))
            .collect();
        cells.shuffle(&mut rng);
        let mut tokens = vec![vec![".".to_string(); width]; height];
        for (agent, &(i, j)) in cells[..n_agents].iter().enumerate() {
            tokens[i][j] = format!("S{agent}");
        }
        for &(i, j) in &cells[n_agents..2 * n_agents] {
            tokens[i][j] = "X".into();
        }
        let mut walls = Vec::new();
        for &(i, j) in &cells[2 * n_agents..] {
            if rng.random_bool(0.25) {
                tokens[i][j] = "@".into();
                walls.push((i, j));
            }
        }
        let map: Vec<String> = tokens.iter().map(|row| row.join(" ")).collect();
        let mut world = world(&map.join("\n"));

        let mut actions: Vec<[f64; 2]> = (0..n_agents).map(|_| STILL).collect();
        for _ in 0..500 {
            for action in actions.iter_mut() {
                if rng.random_bool(0.2) {
                    *action = [rng.random_range(-1.0..=1.0), rng.random_range(-1.0..=1.0)];
                }
            }
            world.tick(&actions).unwrap();
            assert_no_overlap(&world, &walls);
        }
    }
}

#[test]
fn test_stationary_agent_is_not_displaced_by_several_agents() {
    // Agent 2 never moves; the other three wander randomly around it and into each other.
    let mut world = world(
        "
        S0 . .  . .
        .  . .  . .
        .  . S2 . .
        S1 . .  . S3
        X  X X  X .
        ",
    );
    let walls = [];
    let stationary = world.agents_positions()[2];
    let mut rng = StdRng::seed_from_u64(5);
    let mut actions = vec![STILL; 4];
    for t in 0..100 {
        for (agent, action) in actions.iter_mut().enumerate() {
            if agent != 2 && rng.random_bool(0.2) {
                *action = [rng.random_range(-1.0..=1.0), rng.random_range(-1.0..=1.0)];
            }
        }
        world.tick(&actions).unwrap();
        assert_eq!(world.agents_positions()[2], stationary, "tick {t}");
        assert_no_overlap(&world, &walls);
    }
}
