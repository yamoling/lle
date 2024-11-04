mod errors;
mod laser_config;
mod parser_v1;
mod parser_v2;
mod world_config;

pub use errors::ParseError;
pub use parser_v1::parse as parse_v1;
pub use parser_v2::parse as parse_v2;

use super::World;

pub fn parse(file_content: &str) -> Result<World, ParseError> {
    let config = match parse_v2(file_content) {
        Ok(c) => c,
        Err(ParseError::NotV2) => parse_v1(file_content)?,
        Err(other) => return Err(other),
    };
    config.to_world()
}
