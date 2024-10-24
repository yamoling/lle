mod errors;
mod laser_config;
mod parser_v1;
mod parser_v2;
mod world_config;

pub use errors::ParseError;
pub use laser_config::LaserConfig;
pub use parser_v1::parse as parse_v1;
pub use parser_v2::parse as parse_v2;

use super::World;

pub fn parse(world_str: &str) -> Result<World, ParseError> {
    let config = match parse_v2(world_str) {
        Ok(c) => c,
        Err(ParseError::NotV2) => parse_v1(world_str)?,
        Err(other) => return Err(other),
    };
    config.to_world()
}
