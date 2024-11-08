use serde::{Deserialize, Serialize};
use toml;

use crate::{
    core::parsing::{parse_v1, WorldConfig},
    ParseError, Position,
};

use super::{AgentConfig, PositionsConfig, TomlLaserConfig};

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TomlConfig {
    pub width: Option<usize>,
    pub height: Option<usize>,
    pub world_string: Option<String>,
    #[serde(default)]
    pub agents: Vec<AgentConfig>,
    #[serde(default)]
    pub exits: Vec<PositionsConfig>,
    #[serde(default)]
    pub gems: Vec<PositionsConfig>,
    #[serde(default)]
    pub walls: Vec<PositionsConfig>,
    #[serde(default)]
    pub voids: Vec<PositionsConfig>,
    #[serde(default)]
    pub lasers: Vec<TomlLaserConfig>,
}

impl TomlConfig {
    fn complete_with_world_string(&mut self) -> Result<(), ParseError> {
        let world_str = match &self.world_string {
            Some(s) => s,
            None => return Ok(()),
        };
        let config = parse_v1(&world_str)?;
        if let Some(w) = self.width {
            if w != config.width() {
                return Err(ParseError::InconsistentWorldStringWidth {
                    toml_width: w,
                    world_str_width: config.width(),
                });
            }
        } else {
            self.width = Some(config.width());
        }
        if let Some(h) = self.height {
            if h != config.height() {
                return Err(ParseError::InconsistentWorldStringHeight {
                    toml_height: h,
                    world_str_height: config.height(),
                });
            }
        } else {
            self.height = Some(config.height());
        }
        for (agent_num, starts) in config.random_starts().iter().enumerate() {
            if self.agents.len() <= agent_num {
                self.agents.push(AgentConfig::default());
            }
            let positions = starts.iter().map(|pos| PositionsConfig::from(pos));
            self.agents[agent_num].start_positions.extend(positions);
        }

        for pos in config.exits() {
            self.exits.push(PositionsConfig::from(pos));
        }

        for pos in config.walls() {
            self.walls.push(PositionsConfig::from(pos));
        }

        for pos in config.gems() {
            self.gems.push(PositionsConfig::from(pos));
        }
        self.lasers.extend(
            config
                .sources()
                .iter()
                .map(|(pos, laser)| TomlLaserConfig::from_laser_config(laser, *pos)),
        );
        Ok(())
    }

    pub fn to_toml_string(&self) -> String {
        toml::to_string(self).unwrap()
    }
}

fn compute_positions(
    pos_configs: &[PositionsConfig],
    width: usize,
    height: usize,
) -> Result<Vec<Position>, ParseError> {
    let mut res = vec![];
    for pos_config in pos_configs {
        res.extend(pos_config.to_positions(width, height)?);
    }
    Ok(res)
}

pub fn parse(toml_content: &str) -> Result<WorldConfig, ParseError> {
    let data: TomlConfig = match toml::from_str(toml_content) {
        Ok(d) => d,
        Err(e) => {
            let message = e.to_string();
            // There is probably a better way of finding out which key is unknown
            // but I could not find it.
            if message.contains("unknown field") {
                let key = message.split('`').nth(1).unwrap_or("<unknown key>").into();
                return Err(ParseError::UnknownTomlKey { key, message });
            }
            return Err(ParseError::NotV2);
        }
    };
    data.try_into()
}

impl TryInto<WorldConfig> for TomlConfig {
    type Error = ParseError;
    fn try_into(mut self) -> Result<WorldConfig, Self::Error> {
        self.complete_with_world_string()?;
        let width = match self.width {
            Some(w) => w,
            None => return Err(ParseError::EmptyWorld),
        };
        let height = self.height.unwrap();
        let walls_positions = compute_positions(&self.walls, width, height)?;
        let random_start_positions = self
            .agents
            .iter()
            .map(|a| a.compute_start_positions(width, height, &walls_positions))
            .collect::<Result<Vec<_>, _>>()?;
        let source_configs = self.lasers.iter().map(|l| (l.position, l.into())).collect();
        Ok(WorldConfig::new(
            width,
            height,
            compute_positions(&self.gems, width, height)?,
            random_start_positions,
            compute_positions(&self.voids, width, height)?,
            compute_positions(&self.exits, width, height)?,
            walls_positions,
            source_configs,
        ))
    }
}

impl From<&WorldConfig> for TomlConfig {
    fn from(value: &WorldConfig) -> Self {
        let width = value.width();
        let height = value.height();
        let mut agents = vec![];
        for starts in value.random_starts() {
            agents.push(AgentConfig {
                start_positions: starts
                    .iter()
                    .map(|pos| PositionsConfig::from(pos))
                    .collect(),
            })
        }
        let exits = value.exits().iter().map(PositionsConfig::from).collect();
        let gems = value.gems().iter().map(PositionsConfig::from).collect();
        let walls = value.walls().iter().map(PositionsConfig::from).collect();
        let voids = value.voids().iter().map(PositionsConfig::from).collect();
        let lasers = value
            .sources()
            .iter()
            .map(|(pos, laser)| TomlLaserConfig::from_laser_config(laser, *pos))
            .collect();
        Self {
            width: Some(width),
            height: Some(height),
            world_string: None,
            agents,
            exits,
            gems,
            walls,
            voids,
            lasers,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{ParseError, World};

    use super::parse;

    #[test]
    fn invalid_toml_field() {
        let toml_content = r#"
        world_string = "S0 X"
        invalid_field = 25
        "#;
        match parse(toml_content) {
            Err(ParseError::UnknownTomlKey { key, .. }) => {
                assert_eq!(key, "invalid_field");
            }
            other => panic!("Should return a ParseError::UnknownTomlKey instead of {other:?}"),
        }
    }

    #[test]
    fn invalid_toml_subfield() {
        let toml_content = r#"
        world_string = "S0 X"
        [[agents]]
        invalid_subfield = 25
        "#;
        match parse(toml_content) {
            Err(ParseError::UnknownTomlKey { key, .. }) => {
                assert_eq!(key, "invalid_subfield");
            }
            other => panic!("Should return a ParseError::UnknownTomlKey instead of {other:?}"),
        }
    }

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
    fn start_position_in_wall() {
        let toml_content = r#"
world_string="""
@ . .
@ . X
"""
[[agents]]
start_positions = [{i_min=1}]
        "#;
        let world = World::try_from(toml_content).unwrap();
        let starts = world.possible_starts();
        assert_eq!(2, starts[0].len())
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
