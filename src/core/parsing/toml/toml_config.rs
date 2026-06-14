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
#[path = "../../../unit_tests/test_toml_config.rs"]
mod tests;
