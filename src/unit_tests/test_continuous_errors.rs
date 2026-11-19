//! Tests of `ContinuousError`: every variant is reached through the public API of
//! `ContinuousWorld` and `PhysicsParams`.
use crate::continuous::{ContinuousError, ContinuousWorld, ContinuousWorldState, PhysicsParams};
use crate::{MapSpec, ParseError};

/// Two agents, one gem and one laser source (on the cell (1, 2), whose centre is (2.5, 1.5)).
const MAP: &str = "S0 S1 G\nX X L0N";

fn world() -> ContinuousWorld {
    let mut world = ContinuousWorld::try_from(MAP).unwrap();
    world.reset();
    world
}

#[test]
fn unsupported_features_are_listed_in_canonical_order() {
    // The order is always: boxes, lifts, buttons, levers, pressure_plates, layers.
    let cases: [(&str, &[&str]); 3] = [
        (
            "S0 TU0 B0 H0 P0 # S1 X\n;\nX . . . . . . .",
            &[
                "boxes",
                "lifts",
                "buttons",
                "levers",
                "pressure_plates",
                "layers",
            ],
        ),
        (
            "S0 TU0 B0 S1 X\n;\nX . . . .",
            &["lifts", "buttons", "layers"],
        ),
        ("S0 H0 P0 # X", &["boxes", "levers", "pressure_plates"]),
    ];
    for (map, expected) in cases {
        match ContinuousWorld::try_from(map) {
            Err(ContinuousError::UnsupportedFeature { features }) => {
                assert_eq!(features, expected, "{map:?}");
            }
            other => panic!("{map:?}: expected UnsupportedFeature, got {other:?}"),
        }
    }
}

#[test]
fn unsupported_features_are_rejected_by_from_map_and_named_in_the_message() {
    let valid = MapSpec::parse("S0 # X").unwrap().validate().unwrap();
    let error = ContinuousWorld::from_map(&valid, PhysicsParams::default()).unwrap_err();
    assert!(
        matches!(&error, ContinuousError::UnsupportedFeature { features } if features == &["boxes"]),
        "{error:?}"
    );
    assert!(error.to_string().contains("boxes"), "{error}");
}

#[test]
fn invalid_params_are_rejected_by_validate_and_by_construction() {
    let valid = MapSpec::parse(MAP).unwrap().validate().unwrap();
    let invalid = [
        PhysicsParams {
            dt: 0.0,
            ..Default::default()
        },
        // max_speed * dt > agent_radius / 2: tunnelling is possible.
        PhysicsParams {
            max_speed: 10.0,
            ..Default::default()
        },
    ];
    for params in invalid {
        assert!(
            matches!(
                params.validate(),
                Err(ContinuousError::InvalidParams { .. })
            ),
            "{params:?}"
        );
        assert!(
            matches!(
                ContinuousWorld::from_map(&valid, params),
                Err(ContinuousError::InvalidParams { .. })
            ),
            "{params:?}"
        );
    }
}

#[test]
fn wrong_number_of_actions_is_rejected_by_step_and_tick_without_side_effect() {
    let mut world = world();
    let before = world.get_state();
    assert!(matches!(
        world.step(&[[1.0, 0.0]], 5),
        Err(ContinuousError::InvalidNumberOfActions {
            given: 1,
            expected: 2
        })
    ));
    assert!(matches!(
        world.tick(&[[1.0, 0.0]; 3]),
        Err(ContinuousError::InvalidNumberOfActions {
            given: 3,
            expected: 2
        })
    ));
    assert_eq!(world.get_state(), before);
}

#[test]
fn non_finite_action_is_rejected_with_the_agent_id_without_side_effect() {
    let mut world = world();
    let before = world.get_state();
    match world.step(&[[1.0, 0.0], [f64::INFINITY, 0.0]], 5) {
        Err(ContinuousError::InvalidAction { agent_id, action }) => {
            assert_eq!(agent_id, 1);
            assert_eq!(action, [f64::INFINITY, 0.0]);
        }
        other => panic!("expected InvalidAction, got {:?}", other.err()),
    }
    match world.tick(&[[0.0, f64::NAN], [1.0, 0.0]]) {
        Err(ContinuousError::InvalidAction { agent_id, action }) => {
            assert_eq!(agent_id, 0);
            assert!(action[1].is_nan());
        }
        other => panic!("expected InvalidAction, got {other:?}"),
    }
    assert_eq!(world.get_state(), before);
}

#[test]
fn set_state_rejects_inconsistent_states_with_invalid_state() {
    let mut world = world();
    let valid = world.get_state();
    let with = |change: fn(&mut ContinuousWorldState)| {
        let mut state = valid.clone();
        change(&mut state);
        state
    };
    let invalid = [
        // Formerly `RuntimeWorldError::InvalidNumberOfAgents`.
        with(|s| {
            s.positions.pop();
        }),
        with(|s| s.agents_alive.push(true)),
        // Formerly `RuntimeWorldError::InvalidNumberOfGems`.
        with(|s| s.gems_collected.push(false)),
        with(|s| s.sources_enabled.clear()),
        with(|s| s.velocities[1] = [f64::NAN, 0.0]),
        // Agent 0 inside the laser source, then on top of agent 1.
        with(|s| s.positions[0] = [2.5, 1.5]),
        with(|s| s.positions[0] = [1.4, 0.5]),
    ];
    for state in invalid {
        assert!(
            matches!(
                world.set_state(&state),
                Err(ContinuousError::InvalidState { .. })
            ),
            "{state:?}"
        );
        assert_eq!(world.get_state(), valid, "the state must be left untouched");
    }
}

#[test]
fn parse_and_validation_failures_are_wrapped_in_parse() {
    assert!(matches!(
        ContinuousWorld::try_from("S0 X\nS1"),
        Err(ContinuousError::Parse(
            ParseError::Inconsistent2Dimensions { .. }
        ))
    ));
    // Parsing succeeds but the validation fails.
    assert!(matches!(
        ContinuousWorld::try_from("S0 S1 X"),
        Err(ContinuousError::Parse(ParseError::NotEnoughExitTiles {
            n_starts: 2,
            n_exits: 1
        }))
    ));
    assert!(matches!(
        ContinuousWorld::from_file("/this/file/does/not/exist"),
        Err(ContinuousError::Parse(ParseError::InvalidFileName { .. }))
    ));
    // A valid TOML document that is not a valid map.
    assert!(matches!(
        ContinuousWorld::try_from("width = \"wide\"\nheight = 2\nworld_string = \"S0 X\"\n"),
        Err(ContinuousError::Parse(
            ParseError::InvalidTomlDocument { .. }
        ))
    ));
}

#[test]
fn errors_are_reported_in_the_order_params_features_then_parse() {
    // The map has a box and does not validate (2 starts, 1 exit): the unsupported feature wins.
    let spec = MapSpec::parse("S0 S1 # X").unwrap();
    assert!(matches!(
        spec.clone().validate(),
        Err(ParseError::NotEnoughExitTiles { .. })
    ));
    match ContinuousWorld::try_from("S0 S1 # X") {
        Err(ContinuousError::UnsupportedFeature { features }) => assert_eq!(features, ["boxes"]),
        other => panic!("expected UnsupportedFeature, got {other:?}"),
    }
    // Invalid params win over unsupported features.
    let valid = MapSpec::parse("S0 # X").unwrap().validate().unwrap();
    let params = PhysicsParams {
        dt: 0.0,
        ..Default::default()
    };
    assert!(matches!(
        ContinuousWorld::from_map(&valid, params),
        Err(ContinuousError::InvalidParams { .. })
    ));
}

#[test]
fn get_level_out_of_range_is_an_invalid_level() {
    for asked in [0, 7] {
        match ContinuousWorld::get_level(asked) {
            Err(ContinuousError::Parse(ParseError::InvalidLevel { asked: a, min, max })) => {
                assert_eq!((a, min, max), (asked, 1, 6));
            }
            other => panic!("level {asked}: expected InvalidLevel, got {other:?}"),
        }
    }
}

#[test]
fn parse_error_converts_into_continuous_error() {
    assert!(matches!(
        ContinuousError::from(ParseError::NoAgents),
        ContinuousError::Parse(ParseError::NoAgents)
    ));
    // `?` converts a `ParseError` into a `ContinuousError`.
    fn load(content: &str) -> Result<MapSpec, ContinuousError> {
        Ok(MapSpec::parse(content)?)
    }
    assert!(matches!(
        load(""),
        Err(ContinuousError::Parse(ParseError::EmptyWorld))
    ));
    // It is a standard error, usable as `Box<dyn Error>`.
    let boxed: Box<dyn std::error::Error> = Box::new(ContinuousError::from(ParseError::NoAgents));
    assert!(!boxed.to_string().is_empty());
}
