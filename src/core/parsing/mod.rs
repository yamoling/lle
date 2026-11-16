mod actuator_token;
mod button_config;
mod errors;
mod laser_config;
mod lever_config;
mod lift_config;
mod parser_v1;
mod pressure_plate_config;
mod toml;
mod world_config;

pub use errors::ParseError;
pub use parser_v1::parse as parse_v1;
pub use toml::parse as parse_toml;
pub use world_config::WorldConfig;

use super::World;

pub fn parse(file_content: &str) -> Result<World, ParseError> {
    parse_config(file_content)?.into_world()
}

pub(crate) fn parse_config(file_content: &str) -> Result<WorldConfig, ParseError> {
    match parse_toml(file_content) {
        Err(ParseError::NotV2) => parse_v1(file_content),
        other => other,
    }
}
