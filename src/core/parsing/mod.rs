mod errors;
mod laser_config;
mod parser_v1;
mod toml;
mod world_config;

pub use errors::ParseError;
pub use parser_v1::parse as parse_v1;
pub use toml::parse as parse_toml;
pub use world_config::WorldConfig;

use super::World;

pub fn parse(file_content: &str) -> Result<World, ParseError> {
    let config = match parse_toml(file_content) {
        Ok(c) => c,
        Err(ParseError::NotV2) => parse_v1(file_content)?,
        Err(other) => return Err(other),
    };
    config.to_world()
}
