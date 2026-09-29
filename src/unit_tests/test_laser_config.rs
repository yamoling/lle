use crate::tiles::CardinalDirection;

use super::LaserConfig;

#[test]
fn laser_source_from_str() {
    let source = LaserConfig::from_str("L0E", 0).unwrap().build(3);
    assert_eq!(source.direction(), CardinalDirection::East);
    assert_eq!(source.agent_id(), 0);
    assert_eq!(source.laser_id(), 0);

    let source = LaserConfig::from_str("L1W", 25).unwrap().build(5);
    assert_eq!(source.direction(), CardinalDirection::West);
    assert_eq!(source.agent_id(), 1);
    assert_eq!(source.laser_id(), 25);

    let source = LaserConfig::from_str("L2N", 0).unwrap().build(10);
    assert_eq!(source.direction(), CardinalDirection::North);
    assert_eq!(source.agent_id(), 2);
    assert_eq!(source.laser_id(), 0);

    let source = LaserConfig::from_str("L3S", 0).unwrap().build(800);
    assert_eq!(source.direction(), CardinalDirection::South);
    assert_eq!(source.agent_id(), 3);
    assert_eq!(source.laser_id(), 0);
}

#[test]
fn laser_source_from_str_multi_digit_agent_id() {
    let source = LaserConfig::from_str("L12E", 0).unwrap().build(3);
    assert_eq!(source.direction(), CardinalDirection::East);
    assert_eq!(source.agent_id(), 12);
    assert_eq!(source.laser_id(), 0);
}
