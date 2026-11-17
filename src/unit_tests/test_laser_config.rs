use crate::tiles::{CardinalDirection, LaserSource};

use super::LaserConfig;

#[test]
fn laser_source_from_str() {
    let source = LaserSource::from_config(&LaserConfig::from_str("L0E", 0).unwrap(), 3);
    assert_eq!(source.direction(), CardinalDirection::East);
    assert_eq!(source.agent_id(), 0);
    assert_eq!(source.laser_id(), 0);

    let source = LaserSource::from_config(&LaserConfig::from_str("L1W", 25).unwrap(), 5);
    assert_eq!(source.direction(), CardinalDirection::West);
    assert_eq!(source.agent_id(), 1);
    assert_eq!(source.laser_id(), 25);

    let source = LaserSource::from_config(&LaserConfig::from_str("L2N", 0).unwrap(), 10);
    assert_eq!(source.direction(), CardinalDirection::North);
    assert_eq!(source.agent_id(), 2);
    assert_eq!(source.laser_id(), 0);

    let source = LaserSource::from_config(&LaserConfig::from_str("L3S", 0).unwrap(), 800);
    assert_eq!(source.direction(), CardinalDirection::South);
    assert_eq!(source.agent_id(), 3);
    assert_eq!(source.laser_id(), 0);
}

#[test]
fn laser_source_from_str_multi_digit_agent_id() {
    let source = LaserSource::from_config(&LaserConfig::from_str("L12E", 0).unwrap(), 3);
    assert_eq!(source.direction(), CardinalDirection::East);
    assert_eq!(source.agent_id(), 12);
    assert_eq!(source.laser_id(), 0);
}
