use std::{error::Error, fmt::Display};

use crate::{agent::AgentId, Position};

#[derive(Debug)]
pub enum ParseError {
    EmptyWorld,
    NoAgents,
    InvalidTile {
        tile_str: String,
        line: usize,
        col: usize,
    },
    InvalidFileName {
        file_name: String,
    },
    InvalidLevel {
        asked: usize,
        min: usize,
        max: usize,
    },
    NotEnoughExitTiles {
        n_starts: usize,
        n_exits: usize,
    },
    NotEnoughStartTiles {
        n_starts: usize,
        n_agents: usize,
    },
    DuplicateStartTile {
        agent_id: AgentId,
        start1: Position,
        start2: Position,
    },
    InconsistentDimensions {
        expected_n_cols: usize,
        actual_n_cols: usize,
        row: usize,
    },
    InvalidLaserSourceAgentId {
        asked_id: AgentId,
        n_agents: AgentId,
    },
    InvalidAgentId {
        given_agent_id: String,
    },
    InvalidDirection {
        given: String,
        expected: String,
    },
    AgentWithoutStart {
        agent_id: AgentId,
    },
    InconsistentWorldStringWidth {
        toml_width: usize,
        world_str_width: usize,
    },
    InconsistentWorldStringHeight {
        toml_height: usize,
        world_str_height: usize,
    },
    PositionOutOfBounds {
        i: usize,
        j: usize,
    },
    MissingWidth,
    MissingHeight,
    NotV2,
}

impl Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

impl Error for ParseError {}
