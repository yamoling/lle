use serde::{Deserialize, Serialize};
use toml;

use crate::Position;

use super::parse_v1;
use super::{world_config::Config, ParseError};

#[derive(Debug, Deserialize)]
struct ParsingData {
    pub width: Option<usize>,
    pub height: Option<usize>,
    pub world_string: Option<String>,
    pub agents: Option<Vec<AgentConfig>>,
    #[serde(default)]
    pub exits: Vec<PositionsConfig>,
    #[serde(default)]
    pub gems: Vec<PositionsConfig>,
}

impl ParsingData {
    fn build(self) -> Result<Config, ParseError> {
        if let Some(world_str) = &self.world_string {
            Ok(self.build_with_world_string(world_str)?)
        } else {
            Err(ParseError::EmptyWorld)
        }
    }

    fn compute_exits_positions(
        &self,
        width: usize,
        height: usize,
    ) -> Result<Vec<Position>, ParseError> {
        let mut res = vec![];
        for exit in &self.exits {
            res.extend(exit.to_positions(width, height)?);
        }
        Ok(res)
    }

    fn compute_gems_positions(
        &self,
        width: usize,
        height: usize,
    ) -> Result<Vec<Position>, ParseError> {
        let mut res = vec![];
        for gem in &self.gems {
            res.extend(gem.to_positions(width, height)?);
        }
        Ok(res)
    }

    fn build_with_world_string(&self, world_str: &String) -> Result<Config, ParseError> {
        let mut config = parse_v1(&world_str)?;
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
        if let Some(agents) = &self.agents {
            // Add random start positions
            for (agent_id, agent_config) in agents.into_iter().enumerate() {
                let start_positions =
                    agent_config.compute_start_positions(config.width, config.height)?;
                if config.random_start_positions.len() <= agent_id {
                    config.random_start_positions.push(vec![]);
                }
                config.random_start_positions[agent_id].extend(start_positions);
            }
        }

        config
            .exit_positions
            .extend(self.compute_exits_positions(config.width, config.height)?);

        config
            .gem_positions
            .extend(self.compute_gems_positions(config.width, config.height)?);

        Ok(config)
    }
}

#[derive(Deserialize, Debug)]
struct AgentConfig {
    #[serde(default)]
    pub start_positions: Vec<PositionsConfig>,
}

impl AgentConfig {
    fn compute_start_positions(
        &self,
        world_width: usize,
        world_height: usize,
    ) -> Result<Vec<Position>, ParseError> {
        let mut res = vec![];
        for start in &self.start_positions {
            res.extend(start.to_positions(world_width, world_height)?);
        }
        Ok(res)
    }
}

#[derive(Deserialize, Serialize, Debug)]
#[serde(untagged)]
enum PositionsConfig {
    IJ {
        i: usize,
        j: usize,
    },
    Rect {
        #[serde(default)]
        i_min: usize,
        #[serde(default)]
        i_max: Option<usize>,
        #[serde(default)]
        j_min: usize,
        #[serde(default)]
        j_max: Option<usize>,
    },
}

impl PositionsConfig {
    pub fn to_positions(&self, width: usize, height: usize) -> Result<Vec<Position>, ParseError> {
        match self {
            Self::IJ { i, j } => {
                if *i >= height || *j >= width {
                    return Err(ParseError::PositionOutOfBounds { i: *i, j: *j });
                }
                Ok(vec![Position { i: *i, j: *j }])
            }
            Self::Rect {
                i_min,
                i_max,
                j_min,
                j_max,
            } => {
                if *i_min >= height || *j_min >= width {
                    return Err(ParseError::PositionOutOfBounds {
                        i: *i_min,
                        j: *j_min,
                    });
                }
                let mut positions = vec![];
                for i in *i_min..=i_max.unwrap_or(height - 1) {
                    for j in *j_min..=j_max.unwrap_or(width - 1) {
                        if i >= height || j >= width {
                            return Err(ParseError::PositionOutOfBounds { i, j });
                        }
                        positions.push(Position { i, j });
                    }
                }
                Ok(positions)
            }
        }
    }
}

pub fn parse(toml_content: &str) -> Result<Config, ParseError> {
    let data: ParsingData = match toml::from_str(toml_content) {
        Ok(d) => d,
        Err(e) => {
            println!("{e:?}");
            return Err(ParseError::NotV2);
        }
    };
    data.build()
}

#[cfg(test)]
mod tests {
    use crate::{ParseError, World};

    use super::parse;

    #[test]
    fn parse_toml_width_problem() {
        match parse(
            r#"
width = 10
world_string = "S0 X"
"#,
        ) {
            Err(ParseError::InconsistentWorldStringWidth {
                toml_width,
                world_str_width,
            }) => {
                assert_eq!(toml_width, 10);
                assert_eq!(world_str_width, 2);
            }
            _ => panic!("Should return a ParseError::InconsistentWorldStringWidth"),
        }
    }

    #[test]
    fn parse_toml_height_problem() {
        match parse(
            r#"
height = 10
world_string = "S0 X"
"#,
        ) {
            Err(ParseError::InconsistentWorldStringHeight {
                toml_height,
                world_str_height,
            }) => {
                assert_eq!(toml_height, 10);
                assert_eq!(world_str_height, 1);
            }
            _ => panic!("Should return a ParseError::InconsistentWorldStringWidth"),
        }
    }

    #[test]
    fn test_ok() {
        let toml_content = r#"
width = 10
height = 5
exits = [{ j_min = 9 }]
gems = [{ i = 0, j = 2 }]
world_string = """
X . . . S1 . . . . . 
. . . . .  . . . . . 
. . . . .  . . . . . 
. . . . .  . . . . . 
. . . . .  . . . . . 
"""

[[agents]]
start_positions = [{ i_min = 0, i_max = 0 }]

[[agents]]
# Deduced from the string map that agent 1 has a start position at (0, 5).

[[agents]]
start_positions = [{ i = 0, j = 5 }, { i = 3, j = 5 }]

[[agents]]
start_positions = [
    { i = 4, j = 9 },
    { i_min = 1, i_max = 3, j_min = 0, j_max = 3 },
    { j_min = 4 },
]
"#;
        let w = World::try_from(toml_content).unwrap();
        assert_eq!(w.exits_positions().len(), 6);
        assert_eq!(w.gems_positions().len(), 1);
    }
}
