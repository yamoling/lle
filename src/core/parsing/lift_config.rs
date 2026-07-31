use crate::{
    AgentId,
    tiles::{Lift, VerticalDirection},
};

use super::ParseError;

#[derive(Debug)]
pub struct LiftConfig {
    pub direction: VerticalDirection,
    pub group_id: usize,
    pub authorized_agent_id: Option<AgentId>,
}

impl LiftConfig {
    /// Parses a token of the form `T<dir><group_id>` optionally followed by
    /// `A<agent_id>` to restrict the lift to a single agent, e.g. `T2U0` or
    /// `T2U0A1`. `<dir>` must be `U` (up) or `D` (down): a lift only ever moves
    /// an agent between layers.
    pub fn from_str(value: &str) -> Result<LiftConfig, ParseError> {
        let dir_str = value.get(1..2).ok_or_else(|| ParseError::InvalidDirection {
            given: value.to_string(),
            expected: "a direction character following 'T', e.g. TU0".into(),
        })?;
        let direction = VerticalDirection::try_from(dir_str)?;
        let rest = value.get(2..).unwrap_or("");
        let (group_str, authorized_agent_id) = match rest.split_once('A') {
            Some((group_str, agent_str)) => {
                let agent_id = agent_str
                    .parse::<AgentId>()
                    .map_err(|_| ParseError::InvalidAgentId {
                        given_agent_id: agent_str.to_string(),
                    })?;
                (group_str, Some(agent_id))
            }
            None => (rest, None),
        };
        let group_id = group_str
            .parse::<usize>()
            .map_err(|_| ParseError::InvalidGroupId {
                given_group_id: group_str.to_string(),
            })?;
        Ok(Self {
            direction,
            group_id,
            authorized_agent_id,
        })
    }

    pub fn to_string(&self) -> String {
        match self.authorized_agent_id {
            Some(agent_id) => format!(
                "T{}{}A{}",
                self.direction.to_file_string(),
                self.group_id,
                agent_id
            ),
            None => format!("T{}{}", self.direction.to_file_string(), self.group_id),
        }
    }

    pub fn build(&self) -> Lift {
        Lift::new(self.direction, self.authorized_agent_id, self.group_id)
    }
}

impl From<&Lift> for LiftConfig {
    fn from(lift: &Lift) -> Self {
        Self {
            direction: lift.direction(),
            group_id: lift.group_id(),
            authorized_agent_id: lift.authorized_agent_id(),
        }
    }
}

#[cfg(test)]
mod test {
    use super::LiftConfig;
    use crate::tiles::VerticalDirection;

    #[test]
    fn lift_config_from_str_no_agent() {
        let config = LiftConfig::from_str("TU0").unwrap();
        assert_eq!(config.direction, VerticalDirection::Up);
        assert_eq!(config.group_id, 0);
        assert_eq!(config.authorized_agent_id, None);
    }

    #[test]
    fn lift_config_from_str_with_agent() {
        let config = LiftConfig::from_str("TD12A3").unwrap();
        assert_eq!(config.direction, VerticalDirection::Down);
        assert_eq!(config.group_id, 12);
        assert_eq!(config.authorized_agent_id, Some(3));
    }

    #[test]
    fn lift_config_from_str_up_down() {
        let config = LiftConfig::from_str("TU0").unwrap();
        assert_eq!(config.direction, VerticalDirection::Up);

        let config = LiftConfig::from_str("TD0").unwrap();
        assert_eq!(config.direction, VerticalDirection::Down);
    }

    #[test]
    fn lift_config_from_str_invalid_direction() {
        assert!(LiftConfig::from_str("TZ0").is_err());
    }

    #[test]
    fn lift_config_from_str_horizontal_direction_is_invalid() {
        assert!(LiftConfig::from_str("TN0").is_err());
        assert!(LiftConfig::from_str("TE0").is_err());
        assert!(LiftConfig::from_str("TS0").is_err());
        assert!(LiftConfig::from_str("TW0").is_err());
    }

    #[test]
    fn lift_config_from_str_too_short_returns_error() {
        assert!(LiftConfig::from_str("T").is_err());
    }

    #[test]
    fn lift_config_from_str_invalid_group_id() {
        assert!(LiftConfig::from_str("TUx").is_err());
    }

    #[test]
    fn lift_config_from_str_invalid_agent_id() {
        assert!(LiftConfig::from_str("TU0Ax").is_err());
    }

    #[test]
    fn lift_config_round_trip() {
        let config = LiftConfig::from_str("TU0A1").unwrap();
        assert_eq!(config.to_string(), "TU0A1");

        let config = LiftConfig::from_str("TD3").unwrap();
        assert_eq!(config.to_string(), "TD3");
    }
}
