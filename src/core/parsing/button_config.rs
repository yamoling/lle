use crate::{agent::Colour, tiles::Button};

use super::{ParseError, actuator_token::parse_actuator_token};

#[derive(Debug)]
pub struct ButtonConfig {
    pub group_id: usize,
    pub authorized_colour: Option<Colour>,
}

impl ButtonConfig {
    /// Parses a token of the form `B<group_id>` optionally followed by
    /// `A<colour>` to restrict the button to agents of one colour, e.g. `B0` or
    /// `B0A1`. As with `S<c>`, the trailing number is a *colour*, so a restricted
    /// button admits every agent sharing that colour.
    pub fn from_str(value: &str) -> Result<ButtonConfig, ParseError> {
        let (group_id, authorized_colour) = parse_actuator_token(value)?;
        Ok(Self {
            group_id,
            authorized_colour,
        })
    }

    pub fn build(&self) -> Button {
        let button = Button::new(self.group_id);
        match self.authorized_colour {
            Some(colour) => button.restricted_to(colour),
            None => button,
        }
    }
}

impl std::fmt::Display for ButtonConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.authorized_colour {
            Some(colour) => write!(f, "B{}A{}", self.group_id, colour),
            None => write!(f, "B{}", self.group_id),
        }
    }
}

impl From<&Button> for ButtonConfig {
    fn from(button: &Button) -> Self {
        Self {
            group_id: button.group_id(),
            authorized_colour: button.authorized_colour(),
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
        assert_eq!(config.authorized_colour, None);
    }

    #[test]
    fn button_config_from_str_with_agent() {
        let config = ButtonConfig::from_str("B12A3").unwrap();
        assert_eq!(config.group_id, 12);
        assert_eq!(config.authorized_colour, Some(3));
    }

    #[test]
    fn button_config_from_str_multi_digit_colour() {
        // `A` takes a colour, and colours are not capped at one digit: `S10`
        // declares colour 10, so `B0A10` must authorize colour 10 - not 1.
        let config = ButtonConfig::from_str("B0A10").unwrap();
        assert_eq!(config.group_id, 0);
        assert_eq!(config.authorized_colour, Some(10));
    }

    #[test]
    fn button_config_multi_digit_colour_round_trip() {
        let config = ButtonConfig::from_str("B7A42").unwrap();
        assert_eq!(config.group_id, 7);
        assert_eq!(config.authorized_colour, Some(42));
        assert_eq!(config.to_string(), "B7A42");
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
