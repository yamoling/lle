// Wired from `src/continuous/params.rs`.
use crate::{
    ParseError,
    continuous::{ActionModel, ContinuousWorld, PhysicsParams},
    core::parsing::parse_v1,
};

#[test]
fn test_defaults() {
    let params = PhysicsParams::default();
    assert_eq!(params.dt, 0.05);
    assert_eq!(params.agent_radius, 0.3);
    assert_eq!(params.max_speed, 2.0);
    assert_eq!(params.action_model, ActionModel::Velocity);
    assert_eq!(params.max_acceleration, 8.0);
    assert_eq!(params.damping, 0.9);
    assert_eq!(params.gem_radius, 0.25);
    assert_eq!(params.beam_width, 0.1);
    assert!(params.validate().is_ok());
}

#[test]
fn test_anti_tunnelling_invariant() {
    // max_speed * dt <= agent_radius / 2, with values that are exact in binary.
    let at_limit = PhysicsParams {
        dt: 0.0625,
        agent_radius: 0.25,
        max_speed: 2.0,
        ..Default::default()
    };
    assert!(at_limit.validate().is_ok());
    let too_fast = PhysicsParams {
        max_speed: 2.5,
        ..at_limit
    };
    assert!(matches!(
        too_fast.validate(),
        Err(ParseError::InvalidPhysicsParams { .. })
    ));
}

#[test]
fn test_invalid_params_are_rejected() {
    let base = PhysicsParams::default();
    let invalid = [
        PhysicsParams { dt: 0.0, ..base },
        PhysicsParams { dt: -0.05, ..base },
        PhysicsParams {
            agent_radius: 0.0,
            ..base
        },
        PhysicsParams {
            agent_radius: 0.5,
            max_speed: 1.0,
            ..base
        },
        PhysicsParams {
            damping: -0.1,
            ..base
        },
        PhysicsParams {
            damping: 1.1,
            ..base
        },
    ];
    for params in invalid {
        assert!(
            matches!(
                params.validate(),
                Err(ParseError::InvalidPhysicsParams { .. })
            ),
            "{params:?}"
        );
    }
    for damping in [0.0, 1.0] {
        assert!(PhysicsParams { damping, ..base }.validate().is_ok());
    }
}

#[test]
fn test_world_construction_validates_params() {
    let params = PhysicsParams {
        max_speed: 4.0,
        ..Default::default()
    };
    assert!(matches!(
        ContinuousWorld::from_config(&parse_v1("S0 . X").unwrap(), params),
        Err(ParseError::InvalidPhysicsParams { .. })
    ));
}
