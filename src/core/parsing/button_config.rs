use crate::{AgentId, tiles::Button};

use super::ParseError;

#[derive(Debug)]
pub struct ButtonConfig {
    pub group_id: usize,
    pub authorized_agent_id: Option<AgentId>,
}

impl ButtonConfig {
    /// Parses a token of the form `B<group_id>` optionally followed by
    /// `A<agent_id>` to restrict the button to a single agent, e.g. `B0` or
    /// `B0A1`.
    pub fn from_str(value: &str) -> Result<ButtonConfig, ParseError> {
        let rest = &value[1..];
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
            group_id,
            authorized_agent_id,
        })
    }

    pub fn to_string(&self) -> String {
        match self.authorized_agent_id {
            Some(agent_id) => format!("B{}A{}", self.group_id, agent_id),
            None => format!("B{}", self.group_id),
        }
    }

    pub fn build(&self) -> Button {
        let button = Button::new(self.group_id);
        match self.authorized_agent_id {
            Some(agent_id) => button.restricted_to(agent_id),
            None => button,
        }
    }
}

impl From<&Button> for ButtonConfig {
    fn from(button: &Button) -> Self {
        Self {
            group_id: button.group_id(),
            authorized_agent_id: button.authorized_agent_id(),
        }
    }
}

#[cfg(test)]
mod test {
    use super::ButtonConfig;

    #[test]
    fn button_config_from_str_no_agent() {
        let config = ButtonConfig::from_str("B0").unwrap();
        assert_eq!(config.group_id, 0);
        assert_eq!(config.authorized_agent_id, None);
    }

    #[test]
    fn button_config_from_str_with_agent() {
        let config = ButtonConfig::from_str("B12A3").unwrap();
        assert_eq!(config.group_id, 12);
        assert_eq!(config.authorized_agent_id, Some(3));
    }

    #[test]
    fn button_config_from_str_invalid_group_id() {
        assert!(ButtonConfig::from_str("Bx").is_err());
    }

    #[test]
    fn button_config_from_str_invalid_agent_id() {
        assert!(ButtonConfig::from_str("B0Ax").is_err());
    }

    #[test]
    fn button_config_round_trip() {
        let config = ButtonConfig::from_str("B0A1").unwrap();
        assert_eq!(config.to_string(), "B0A1");

        let config = ButtonConfig::from_str("B3").unwrap();
        assert_eq!(config.to_string(), "B3");
    }
}
