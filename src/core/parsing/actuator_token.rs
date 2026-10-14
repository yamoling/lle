//! Token syntax shared by the tiles that pulse lifts: `Button`, `Lever` and `PressurePlate`.

use crate::agent::Colour;

use super::ParseError;

/// Parses the `<group_id>[A<colour>]` part shared by every actuator token
/// (`B` button, `H` lever, `P` pressure plate), after its one-letter prefix.
pub(super) fn parse_actuator_token(value: &str) -> Result<(usize, Option<Colour>), ParseError> {
    let rest = &value[1..];
    let (group_str, authorized_colour) = match rest.split_once('A') {
        Some((group_str, colour_str)) => {
            let colour = colour_str
                .parse::<Colour>()
                .map_err(|_| ParseError::InvalidAgentId {
                    given_agent_id: colour_str.to_string(),
                })?;
            (group_str, Some(colour))
        }
        None => (rest, None),
    };
    let group_id = group_str
        .parse::<usize>()
        .map_err(|_| ParseError::InvalidGroupId {
            given_group_id: group_str.to_string(),
        })?;
    Ok((group_id, authorized_colour))
}
