use serde::Deserialize;
use toml;

use crate::Position;

use super::parse_v1;
use super::{world_config::Config, ParseError};

#[derive(Debug, Deserialize)]
struct ParsingData {
    pub width: Option<usize>,
    pub height: Option<usize>,
    pub n_agents: Option<u32>,
    pub map_string: Option<String>,
    pub agents: Vec<AgentConfig>,
}

impl ParsingData {
    fn build(self) -> Result<Config, ParseError> {
        if let Some(world_str) = &self.map_string {
            let config = parse_v1(&world_str)?;
            Ok(self.from_config(config)?)
        } else {
            self.into_config()
        }
    }

    fn into_config(self) -> Result<Config, ParseError> {
        todo!()
    }

    fn from_config(self, mut config: Config) -> Result<Config, ParseError> {
        if let Some(w) = self.width {
            if w != config.width {
                return Err(ParseError::InconsistentWorldStringWidth {
                    toml_width: w,
                    world_str_width: config.width,
                });
            }
        }
        if let Some(h) = self.height {
            if h != config.height {
                return Err(ParseError::InconsistentWorldStringHeight {
                    toml_height: h,
                    world_str_height: config.height,
                });
            }
        }
        // Add random start positions
        for (agent_id, agent_config) in self.agents.into_iter().enumerate() {
            let start_positions = agent_config.get(config.width, config.height);
            config.random_start_positions[agent_id].extend(start_positions);
        }
        Ok(config)
    }
}

#[derive(Deserialize, Debug)]
struct AgentConfig {
    pub start_positions: Option<PositionsConfig>,
}

impl AgentConfig {
    fn get(&self, world_width: usize, world_height: usize) -> Vec<Position> {
        match &self.start_positions {
            Some(positions) => positions.to_positions(world_width, world_height),
            None => vec![],
        }
    }
}

#[derive(Deserialize, Debug, Default)]
struct PositionsConfig {
    #[serde(default)]
    pub rectangles: Vec<Rectangle>,
    #[serde(default)]
    pub positions: Vec<Position>,
}

impl PositionsConfig {
    pub fn to_positions(&self, width: usize, height: usize) -> Vec<Position> {
        let mut positions = self.positions.clone();
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
    pub i_min: usize,
    pub i_max: Option<usize>,
    #[serde(default)]
    pub j_min: usize,
    pub j_max: Option<usize>,
}

pub fn parse(toml_content: &str) -> Result<Config, ParseError> {
    let data: ParsingData = match toml::from_str(toml_content) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("{e:?}");
            return Err(ParseError::NotV2);
        }
    };
    println!("{:?}", data);
    data.build()
}
