use crate::{WorldEvent, continuous::ContinuousWorld};

const STILL: [f64; 2] = [0.0, 0.0];
const NORTH: [f64; 2] = [0.0, -1.0];

fn world(map: &str) -> ContinuousWorld {
    let mut world = ContinuousWorld::try_from(map).unwrap();
    world.reset();
    world
}

fn run(world: &mut ContinuousWorld, actions: &[[f64; 2]], n_ticks: usize) -> Vec<WorldEvent> {
    (0..n_ticks)
        .flat_map(|_| world.tick(actions).unwrap())
        .collect()
}

/// Puts the agents at `positions` and runs one tick without moving.
fn place_and_tick(world: &mut ContinuousWorld, positions: &[[f64; 2]]) -> Vec<WorldEvent> {
    let mut state = world.get_state();
    state.positions = positions.to_vec();
    world.set_state(&state).unwrap();
    world.tick(&vec![STILL; positions.len()]).unwrap()
}

/// The L0 beam runs east along y = 1.5, from x = 1 to the grid edge.
const MIDDLE_BEAM: &str = "
    S0  S1 . . . X
    L0E .  . . . .
    .   .  . . . X
";

#[test]
fn test_different_colour_agent_dies_when_its_disc_touches_the_beam() {
    // Agent 1 walks north towards the beam at y = 0.5. It overlaps the beam when
    // y < 0.5 + r + beam_width / 2 = 0.85, i.e. at the 7th tick (y = 0.8).
    let mut world = world(
        "
        L0E .  .  X
        S0  S1 .  X
        ",
    );
    assert!(run(&mut world, &[STILL, NORTH], 6).is_empty());
    assert!(world.get_state().agents_alive[1]);
    assert_eq!(
        world.tick(&[STILL, NORTH]).unwrap(),
        vec![WorldEvent::AgentDied { agent_id: 1 }]
    );
}

#[test]
fn test_beam_overlap_threshold() {
    // Overlap iff |h| < r + beam_width / 2 = 0.35.
    let mut world = world(MIDDLE_BEAM);
    assert!(place_and_tick(&mut world, &[[0.5, 0.5], [3.5, 1.5 + 0.36]]).is_empty());
    assert_eq!(
        place_and_tick(&mut world, &[[0.5, 0.5], [3.5, 1.5 + 0.34]]),
        vec![WorldEvent::AgentDied { agent_id: 1 }]
    );
}

#[test]
fn test_same_colour_agent_cuts_the_beam_and_protects_downstream_agents() {
    // Agent 0 blocks the L0 beam (y = 0.5) in front of agent 1. Agent 0 leaves the beam when
    // y > 0.85, at its 4th tick southwards (y = 0.9).
    let mut world = world(
        "
        L0E S0 . S1 X
        .   .  . .  X
        ",
    );
    assert!(run(&mut world, &[STILL, STILL], 5).is_empty());
    assert!(run(&mut world, &[[0.0, 1.0], STILL], 3).is_empty());
    assert_eq!(
        world.tick(&[[0.0, 1.0], STILL]).unwrap(),
        vec![WorldEvent::AgentDied { agent_id: 1 }]
    );
}

#[test]
fn test_grazing_blocker_cuts_the_beam() {
    // Agent 0 only touches the beam with the edge of its disc (|h| = 0.34).
    let mut world = world(MIDDLE_BEAM);
    let events = place_and_tick(&mut world, &[[2.5, 1.5 - 0.34], [3.0, 1.5]]);
    assert!(events.is_empty());
}

#[test]
fn test_cut_point_is_the_first_contact_of_the_disc() {
    // The centre of agent 1 is downstream of the centre of the blocker, but its disc meets the
    // beam first: s_first = s - sqrt(r² - (|h| - w/2)²) is 2.5 - 0.077 for agent 0 (|h| = 0.34)
    // and 2.55 - 0.166 for agent 1 (|h| = 0.30).
    let mut world = world(MIDDLE_BEAM);
    assert_eq!(
        place_and_tick(&mut world, &[[2.5, 1.5 - 0.34], [2.55, 1.5 + 0.30]]),
        vec![WorldEvent::AgentDied { agent_id: 1 }]
    );
}

#[test]
fn test_disabled_source_never_kills() {
    let mut world = world("L0E . .  X\nS0  . S1 X");
    assert_eq!(
        run(&mut world, &[STILL, NORTH], 7),
        vec![WorldEvent::AgentDied { agent_id: 1 }]
    );

    world.reset();
    let mut state = world.get_state();
    state.sources_enabled = vec![false];
    world.set_state(&state).unwrap();
    assert!(run(&mut world, &[STILL, NORTH], 15).is_empty());
    assert!(run(&mut world, &[STILL, [-1.0, 0.0]], 20).is_empty());
    assert!(world.get_state().agents_alive[1]);
    // The source square stays solid: agent 1 stops against it, at x = 1 + r.
    let [x, y] = world.agents_positions()[1];
    assert!((x - 1.3).abs() < 1e-9 && (y - 0.3).abs() < 1e-9, "{x}, {y}");
}

#[test]
fn test_cascade_exposes_the_agent_behind_a_dead_blocker_in_the_same_tick() {
    // Agent 0 blocks L0 (row 2) in front of agent 1. Walking east, it enters the L1 beam
    // (column 2, x = 2.5) when x > 2.15, at its 7th tick (x = 2.2).
    let mut world = world(
        "
        .   .  L1S .  .
        .   .  .   .  .
        L0E S0 .   S1 X
        .   .  .   .  X
        ",
    );
    assert!(run(&mut world, &[[1.0, 0.0], STILL], 6).is_empty());
    let mut events = world.tick(&[[1.0, 0.0], STILL]).unwrap();
    events.sort_by_key(|e| format!("{e:?}"));
    assert_eq!(
        events,
        vec![
            WorldEvent::AgentDied { agent_id: 0 },
            WorldEvent::AgentDied { agent_id: 1 },
        ]
    );
}

#[test]
fn test_agent_colour_comes_from_the_toml_config() {
    // Agent 1 has colour 0: it walks onto the L0 beam safely.
    let mut world = world(
        r#"
world_string = """
S0  .  . X
L0E .  . .
.   S1 . X
"""
[[agents]]
colour = 0
[[agents]]
colour = 0
"#,
    );
    assert!(run(&mut world, &[STILL, NORTH], 10).is_empty());
    assert!((world.agents_positions()[1][1] - 1.5).abs() < 1e-9);
}

#[test]
fn test_arrived_agent_still_blocks_its_beam() {
    // Agent 0 walks along its own beam into the exit (arrives at x = 2.06), then agent 1 walks
    // north onto the beam downstream.
    let mut world = world(
        "
        L0E S0 X . .
        .   .  . S1 X
        ",
    );
    assert_eq!(
        run(&mut world, &[[0.7, 0.0], STILL], 8),
        vec![WorldEvent::AgentExit { agent_id: 0 }]
    );
    assert!(run(&mut world, &[STILL, NORTH], 15).is_empty());
    assert!(world.agents_positions()[1][1] < 0.85);
}
