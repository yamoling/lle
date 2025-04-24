use serde::{Deserialize, Serialize};
use toml;

use crate::{
    ParseError, Position,
    core::parsing::{WorldConfig, parse_v1},
};

use super::{AgentConfig, PositionsConfig, TomlLaserConfig};

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TomlConfig {
    pub width: Option<usize>,
    pub height: Option<usize>,
    pub n_agents: Option<usize>,
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
    #[serde(default)]
    pub starts: Vec<PositionsConfig>,
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
            self.agents[agent_num].starts.extend(positions);
        }
        if let Some(n) = self.n_agents {
            if n < self.agents.len() {
                return Err(ParseError::InconsistentNumberOfAgents {
                    toml_n_agents_field: n,
                    actual_n_agents: self.agents.len(),
                });
            }
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
        if let Some(n) = self.n_agents {
            while n > self.agents.len() {
                self.agents.push(AgentConfig::default());
            }
        }
        self.complete_with_world_string()?;
        let width = match self.width {
            Some(w) => w,
            None => return Err(ParseError::EmptyWorld),
        };
        let height = match self.height {
            Some(h) => h,
            None => return Err(ParseError::EmptyWorld),
        };
        let starts_positions = compute_positions(&self.starts, width, height)?;
        let walls_positions = compute_positions(&self.walls, width, height)?;
        let exit_positions = compute_positions(&self.exits, width, height)?;
        let agents_random_start_positions = self
            .agents
            .iter()
            .map(|a| {
                a.compute_start_positions(
                    &starts_positions,
                    width,
                    height,
                    &walls_positions,
                    &exit_positions,
                )
            })
            .collect::<Result<Vec<_>, _>>()?;
        let source_configs = self.lasers.iter().map(|l| (l.position, l.into())).collect();
        Ok(WorldConfig::new(
            width,
            height,
            compute_positions(&self.gems, width, height)?,
            agents_random_start_positions,
            compute_positions(&self.voids, width, height)?,
            exit_positions,
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
                starts: starts
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
            n_agents: Some(agents.len()),
            world_string: None,
            agents,
            exits,
            gems,
            walls,
            voids,
            lasers,
            starts: vec![],
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{ParseError, Position, World};

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
    fn parse_start_pos_rows() {
        match parse(
            r#"
height = 10
width = 10
n_agents = 2
starts = [{row = 0}]
"#,
        ) {
            Ok(config) => {
                // Start positions should be (0, 0), (0, 1) ... (0, 9)
                for start in config.random_starts() {
                    for j in 0..config.width() {
                        assert!(start.contains(&Position { i: 0, j }));
                    }
                }
            }
            _ => panic!("The TOML should be parsed successfully"),
        }
    }

    #[test]
    fn parse_start_pos_cols() {
        match parse(
            r#"
height = 10
width = 10
n_agents = 2
starts = [{col = 0}]
"#,
        ) {
            Ok(config) => {
                for start in config.random_starts() {
                    for i in 0..config.height() {
                        assert!(start.contains(&Position { i, j: 0 }));
                    }
                }
            }
            _ => panic!("The TOML should be parsed successfully"),
        }
    }

    #[test]
    fn parse_start_pos_rows_and_cols() {
        match parse(
            r#"
height = 10
width = 10
n_agents = 2
starts = [{col = 0}, {row=0}]
"#,
        ) {
            Ok(config) => {
                for start in config.random_starts() {
                    assert_eq!(
                        start.len(),
                        19,
                        "There should only be 19 starts since duplicates should be removed"
                    );
                    for i in 0..config.height() {
                        assert!(start.contains(&Position { i, j: 0 }));
                    }
                    for j in 0..config.width() {
                        assert!(start.contains(&Position { i: 0, j }));
                    }
                }
            }
            _ => panic!("The TOML should be parsed successfully"),
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
        assert_eq!(1, starts[0].len())
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

    #[test]
    fn test_global_start_pos() {
        let toml_content = r#"
width = 10
height = 10
n_agents = 5
starts = [{ row = 0 }]
exits = [{ col = 4 }]
"#;
        let w = World::try_from(toml_content).unwrap();
        assert_eq!(w.exits_positions().len(), 10);
        for starts in w.possible_starts() {
            assert_eq!(starts.len(), 9);
        }
    }
}
