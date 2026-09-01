//! TDD tests for `.agents/plans/agent-colour-id.md`: agent ID and agent colour become
//! independent, and laser blocking is decided by colour.
//!
//! These tests fail today. They cover what needs the new API (`Agent::colour`, `World::n_colours`,
//! `LaserSource::colour`, the TOML `colour` key); everything expressible through the current
//! `World::try_from` signature — repeated `S<c>` tokens, colour-major ids, colour-based blocking —
//! lives in `test_world.rs` instead.

use crate::{Action, Position, World, tiles::Laser};

fn pos(i: usize, j: usize) -> Position {
    Position { i, j }
}

fn get_laser(world: &World, pos: Position) -> &Laser {
    for (p, laser) in world.lasers() {
        if pos == p {
            return laser;
        }
    }
    panic!("No laser at position {pos:?}");
}

/// Agent 0 starts top-left, agent 1 below the beam. The `L0E` source at (1, 0) beams east over
/// (1, 1), (1, 2) and (1, 3).
const LAYOUT: &str = "
S0 .  .  X
L0E .  .  .
.   S1 .  X
";

/// Same layout, but both agents are declared with colour 0.
const SHARED_COLOUR: &str = r#"
world_string = """
S0 .  .  X
L0E .  .  .
.   S1 .  X
"""
[[agents]]
colour = 0
[[agents]]
colour = 0
"#;

/// Sparse colour space: colours {0, 2} over two agents, and a laser of colour 2.
const SPARSE_COLOURS: &str = r#"
world_string = """
S0 .  .  X
L2E .  .  .
.   S1 .  X
"""
[[agents]]
colour = 0
[[agents]]
colour = 2
"#;

#[test]
fn agent_colour_defaults_to_id() {
    let world = World::try_from(LAYOUT).unwrap();
    for (id, agent) in world.agents().iter().enumerate() {
        assert_eq!(
            agent.colour(),
            id,
            "Without an explicit declaration, an agent's colour must equal its id"
        );
    }
}

#[test]
fn laser_source_exposes_its_colour() {
    let world = World::try_from(LAYOUT).unwrap();
    let (_, source) = world.sources().next().unwrap();
    assert_eq!(source.colour(), 0);
}

#[test]
fn declared_colours_are_parsed() {
    let world = World::try_from(SHARED_COLOUR).unwrap();
    assert_eq!(world.n_agents(), 2);
    assert_eq!(world.agents()[0].colour(), 0);
    assert_eq!(
        world.agents()[1].colour(),
        0,
        "Agent 1 was declared with colour 0"
    );
}

#[test]
fn same_colour_agent_blocks_and_survives_beam() {
    let mut world = World::try_from(SHARED_COLOUR).unwrap();
    world.reset();
    assert!(get_laser(&world, pos(1, 1)).is_on());

    let events = world.step(&[Action::Stay, Action::North]).unwrap();

    assert!(
        events.is_empty(),
        "Agent 1 shares the beam's colour and must survive it, got {events:?}"
    );
    assert!(world.agents()[1].is_alive());
    assert_eq!(world.agents_positions()[1], pos(1, 1));
    assert!(
        get_laser(&world, pos(1, 1)).is_off(),
        "An agent of the beam's colour must switch the beam off"
    );
    assert!(
        get_laser(&world, pos(1, 2)).is_off(),
        "Blocking a beam tile must also switch off every downstream tile"
    );
}

#[test]
fn same_colour_agent_crosses_the_whole_beam() {
    let mut world = World::try_from(SHARED_COLOUR).unwrap();
    world.reset();
    // (2, 1) -> (1, 1) -> (1, 2) -> (1, 3): the whole beam, tile by tile.
    for action in [Action::North, Action::East, Action::East] {
        let events = world.step(&[Action::Stay, action]).unwrap();
        assert!(events.is_empty(), "Unexpected events: {events:?}");
        assert!(world.agents()[1].is_alive());
    }
    assert_eq!(world.agents_positions()[1], pos(1, 3));
}

#[test]
fn n_colours_spans_the_largest_declared_colour() {
    let world = World::try_from(SPARSE_COLOURS).unwrap();
    // Colours are {0, 2}: the colour space is sparse, but observation bands are indexed by
    // colour, so `n_colours` must cover index 2.
    assert_eq!(world.n_colours(), 3);
    // `n_laser_colours` keeps its meaning: how many *distinct* laser colours exist.
    assert_eq!(world.n_laser_colours(), 1);
}

#[test]
fn shared_colour_world_round_trips_through_world_string() {
    let world = World::try_from(SHARED_COLOUR).unwrap();
    let round_tripped = World::try_from(world.world_string()).unwrap();
    let colours: Vec<_> = round_tripped.agents().iter().map(|a| a.colour()).collect();
    assert_eq!(
        colours,
        vec![0, 0],
        "`world_string()` must serialize the colour assignment (here v1 suffices: the two \
         colour-0 agents are id-ordered by reading order, see plan §3.4d)"
    );
}

#[test]
fn default_colour_world_round_trips_unchanged() {
    // Regression guard: worlds that never mention colours must keep emitting v1 ASCII.
    let world = World::try_from(LAYOUT).unwrap();
    let string = world.world_string();
    assert!(
        !string.contains("colour"),
        "A default-colour world must still serialize to the v1 format, got:\n{string}"
    );
    let round_tripped = World::try_from(string).unwrap();
    assert_eq!(round_tripped.agents()[1].colour(), 1);
}

#[test]
fn start_pruning_respects_colour_not_id() {
    // Agent 1 is colour 0 and its only start is on the colour-0 beam: it survives there, so the
    // start must be kept rather than pruned as a lethal tile.
    let world = World::try_from(
        r#"
world_string = """
S0  .  X
L0E S1 X
"""
[[agents]]
colour = 0
[[agents]]
colour = 0
"#,
    )
    .unwrap();
    assert_eq!(world.possible_starts()[1], vec![pos(1, 1)]);
}

// ---------------------------------------------------------------------------
// Phase 2b: repeated `S<c>` tokens in the v1 ASCII format (plan §3.4).
// ---------------------------------------------------------------------------

#[test]
fn sparse_colour_tokens_yield_sparse_colours() {
    // `S0 S2` is `ParseError::AgentWithoutStart` today; colours being independent makes it a
    // well-formed two-agent world with colours {0, 2}.
    let world = World::try_from("S0 S2 X X").unwrap();
    assert_eq!(world.n_agents(), 2);
    assert_eq!(
        world.agents().iter().map(|a| a.colour()).collect::<Vec<_>>(),
        vec![0, 2]
    );
    assert_eq!(world.n_colours(), 3);
}

#[test]
fn v1_round_trip_preserves_colours() {
    let world = World::try_from("S0 S0 X X").unwrap();
    let string = world.world_string();
    assert!(
        string.contains("S0") && !string.contains("colour"),
        "A shared-colour world with single starts is expressible in v1, got:\n{string}"
    );
    let round_tripped = World::try_from(string).unwrap();
    assert_eq!(
        round_tripped
            .agents()
            .iter()
            .map(|a| a.colour())
            .collect::<Vec<_>>(),
        vec![0, 0]
    );
    assert_eq!(round_tripped.possible_starts(), world.possible_starts());
}

#[test]
fn v1_emission_refuses_lossy_id_order() {
    // Agents 0 and 1 share colour 0, but agent 0 starts *after* agent 1 in reading order. Emitting
    // v1 would swap them on reparse, so `world_string()` must fall back to TOML.
    let world = World::try_from(
        r#"
world_string = """
S1 S0 X X
"""
[[agents]]
colour = 0
[[agents]]
colour = 0
"#,
    )
    .unwrap();
    let round_tripped = World::try_from(world.world_string()).unwrap();
    assert_eq!(round_tripped.possible_starts(), world.possible_starts());
}

