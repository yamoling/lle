use serde::Deserialize;
use toml;

use super::{world_config::Config, ParseError};

struct TomlData {
    pub width: u32,
    pub height: u32,
    pub n_agents: u32,
    pub map_string: Option<String>,
    pub agents: Vec<AgentConfig>,
}

#[derive(Debug, Deserialize)]
struct PartialConfig {
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub n_agents: Option<u32>,
    pub map_string: Option<String>,
    pub agents: Vec<AgentConfig>,
}

impl PartialConfig {
    pub fn from_str(toml_content: &str) -> Result<Self, ParseError> {
        let config: Self = toml::from_str(toml_content).unwrap();
        Ok(config)
    }

    pub fn from_file(path: &str) -> Result<Self, ParseError> {
        let toml_content = std::fs::read_to_string(path).unwrap();
        Self::from_str(&toml_content)
    }
}

#[derive(Deserialize, Debug)]
struct AgentConfig {
    pub start_positions: Option<PositionsConfig>,
}

#[derive(Deserialize, Debug)]
struct PositionsConfig {
    #[serde(default)]
    pub rectangles: Vec<Rectangle>,
    #[serde(default)]
    pub positions: Vec<Position>,
}

impl PositionsConfig {
    pub fn to_positions(self, width: u32, height: u32) -> Vec<Position> {
        let mut positions = self.positions;
        for rectangle in &self.rectangles {
            for x in rectangle.i_min..=rectangle.i_max.unwrap_or(height) {
                for y in rectangle.j_min..=rectangle.j_max.unwrap_or(width) {
                    positions.push(Position { i: x, j: y });
                }
            }
        }
        positions
    }
}

#[derive(Deserialize, Debug)]
struct Rectangle {
    #[serde(default)]
    pub i_min: u32,
    pub i_max: Option<u32>,
    #[serde(default)]
    pub j_min: u32,
    pub j_max: Option<u32>,
}

#[derive(Deserialize, Debug)]
struct Position {
    pub i: u32,
    pub j: u32,
}

pub fn parse(toml_content: &str) -> Result<Config, ParseError> {
    let config: PartialConfig = toml::from_str(&toml_content).map_err(|_| ParseError::NotV2)?;
    println!("{:?}", config);
    todo!()
}
