use serde::{Deserialize, Serialize};
use toml;

use crate::{
    ParseError, Position,
    core::parsing::{WorldConfig, parse_v1},
};

use super::{AgentConfig, PositionsConfig, TomlButtonConfig, TomlLaserConfig, TomlLiftConfig};

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TomlConfig {
    pub width: Option<usize>,
    pub height: Option<usize>,
    pub layers: Option<usize>,
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
    pub boxes: Vec<PositionsConfig>,
    #[serde(default)]
    pub lasers: Vec<TomlLaserConfig>,
    #[serde(default)]
    pub starts: Vec<PositionsConfig>,
    #[serde(default)]
    pub lifts: Vec<TomlLiftConfig>,
    #[serde(default)]
    pub buttons: Vec<TomlButtonConfig>,
}
impl TomlConfig {
    fn complete_with_world_string(&mut self) -> Result<(), ParseError> {
        let world_str = match &self.world_string {
            Some(s) => s,
            None => return Ok(()),
        };
        let config = parse_v1(world_str)?;
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
        if let Some(l) = self.layers {
            if l != config.layers() {
                return Err(ParseError::InconsistentWorldStringLayers {
                    toml_layers: l,
                    world_str_layers: config.layers(),
                });
            }
        } else {
            self.layers = Some(config.layers());
        }

        for (agent_num, starts) in config.random_starts().iter().enumerate() {
            if self.agents.len() <= agent_num {
                self.agents.push(AgentConfig::default());
            }
            let positions = starts.iter().map(PositionsConfig::from);
            self.agents[agent_num].starts.extend(positions);
            // The `S<c>` token supplies the agent's colour, unless `[[agents]] colour`
            // states one explicitly — the explicit declaration always wins.
            if self.agents[agent_num].colour.is_none() {
                self.agents[agent_num].colour = Some(config.colours()[agent_num]);
            }
        }
        if let Some(n) = self.n_agents
            && n < self.agents.len()
        {
            return Err(ParseError::InconsistentNumberOfAgents {
                toml_n_agents_field: n,
                actual_n_agents: self.agents.len(),
            });
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

        // Voids declared in the world string must be kept too: they are needed to validate
        // the boxes and they are part of the world.
        for pos in config.voids() {
            self.voids.push(PositionsConfig::from(pos));
        }

        for pos in config.boxes() {
            self.boxes.push(PositionsConfig::from(pos));
        }
        self.lasers.extend(
            config
                .sources()
                .iter()
                .map(|(pos, laser)| TomlLaserConfig::from_laser_config(laser, *pos)),
        );
        self.lifts.extend(
            config
                .lifts()
                .iter()
                .map(|(pos, lift)| TomlLiftConfig::from_lift_config(lift, *pos)),
        );
        self.buttons.extend(
            config
                .buttons()
                .iter()
                .map(|(pos, button)| TomlButtonConfig::from_button_config(button, *pos)),
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
    layers: usize,
) -> Result<Vec<Position>, ParseError> {
    let mut res = vec![];
    for pos_config in pos_configs {
        res.extend(pos_config.to_positions(width, height, layers)?);
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
            // If the content isn't valid TOML at all, it might be a v1 plain-text
            // map string, so let the caller fall back to the v1 parser. If it IS
            // valid TOML but fails to match our schema, surface a clear TOML error
            // instead of a confusing v1 parse error.
            if toml::from_str::<toml::Value>(toml_content).is_err() {
                return Err(ParseError::NotV2);
            }
            return Err(ParseError::InvalidTomlDocument { message });
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
        let layer = match self.layers {
            Some(l) => l,
            None => 1, // if layers is not specified, we assume there is only one layer
        };
        let starts_positions = compute_positions(&self.starts, width, height, layer)?;
        let walls_positions = compute_positions(&self.walls, width, height, layer)?;
        let exit_positions = compute_positions(&self.exits, width, height, layer)?;
        let agents_random_start_positions = self
            .agents
            .iter()
            .map(|a| {
                a.compute_start_positions(
                    &starts_positions,
                    width,
                    height,
                    layer,
                    &walls_positions,
                    &exit_positions,
                )
            })
            .collect::<Result<Vec<_>, _>>()?;
        let source_configs = self.lasers.iter().map(|l| (l.position, l.into())).collect();
        let lift_configs = self.lifts.iter().map(|l| (l.position, l.into())).collect();
        let button_configs = self
            .buttons
            .iter()
            .map(|b| (b.position, b.into()))
            .collect();
        // An agent without a declared colour keeps the historical default: colour = agent id.
        let colours = self
            .agents
            .iter()
            .enumerate()
            .map(|(agent_id, agent)| agent.colour.unwrap_or(agent_id))
            .collect();
        Ok(WorldConfig::new(
            width,
            height,
            layer,
            compute_positions(&self.gems, width, height, layer)?,
            agents_random_start_positions,
            compute_positions(&self.voids, width, height, layer)?,
            exit_positions,
            walls_positions,
            compute_positions(&self.boxes, width, height, layer)?,
            source_configs,
            lift_configs,
            button_configs,
            colours,
        ))
    }
}

impl From<&WorldConfig> for TomlConfig {
    fn from(value: &WorldConfig) -> Self {
        let width = value.width();
        let height = value.height();
        let layers = value.layers();
        let mut agents = vec![];
        for (agent_id, starts) in value.random_starts().iter().enumerate() {
            agents.push(AgentConfig {
                starts: starts.iter().map(PositionsConfig::from).collect(),
                colour: value.colours().get(agent_id).copied(),
            })
        }
        let exits = value.exits().iter().map(PositionsConfig::from).collect();
        let gems = value.gems().iter().map(PositionsConfig::from).collect();
        let walls = value.walls().iter().map(PositionsConfig::from).collect();
        let voids = value.voids().iter().map(PositionsConfig::from).collect();
        let boxes = value.boxes().iter().map(PositionsConfig::from).collect();
        let lasers = value
            .sources()
            .iter()
            .map(|(pos, laser)| TomlLaserConfig::from_laser_config(laser, *pos))
            .collect();
        let lifts = value
            .lifts()
            .iter()
            .map(|(pos, lift)| TomlLiftConfig::from_lift_config(lift, *pos))
            .collect();
        let buttons = value
            .buttons()
            .iter()
            .map(|(pos, button)| TomlButtonConfig::from_button_config(button, *pos))
            .collect();

        Self {
            width: Some(width),
            height: Some(height),
            layers: Some(layers),
            n_agents: Some(agents.len()),
            world_string: None,
            agents,
            exits,
            gems,
            walls,
            voids,
            boxes,
            lasers,
            starts: vec![],
            lifts,
            buttons,
        }
    }
}

#[cfg(test)]
#[path = "../../../unit_tests/test_toml_config.rs"]
mod tests;
