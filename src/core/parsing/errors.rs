use std::{error::Error, fmt::Display};

use pyo3::{PyErr, PyResult, exceptions};

use crate::{
    Position,
    agent::AgentId,
    bindings::{InvalidLevelError, ParsingError},
};

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
    InconsistentNumberOfAgents {
        toml_n_agents_field: usize,
        actual_n_agents: usize,
    },
    PositionOutOfBounds {
        i: usize,
        j: usize,
    },
    MissingWidth,
    MissingHeight,
    UnknownTomlKey {
        key: String,
        message: String,
    },
    NotV2,
}

impl Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

impl Error for ParseError {}

impl<T> Into<PyResult<T>> for ParseError {
    fn into(self) -> PyResult<T> {
        if let ParseError::InvalidFileName { file_name } = self {
            return Err(exceptions::PyFileNotFoundError::new_err(file_name));
        }
        if let ParseError::InvalidLevel { asked, min, max } = self {
            return Err(InvalidLevelError::new_err(format!(
                "Invalid level: {asked}. Expected a level between {min} and {max}.",
                asked = asked,
                min = min,
                max = max
            )));
        }
        let msg = match self {
            ParseError::DuplicateStartTile {
                agent_id,
                start1,
                start2,
            } => format!("Agent {agent_id} has two start tiles: {start1:?} and {start2:?}"),
            ParseError::InconsistentDimensions {
                expected_n_cols,
                actual_n_cols,
                row,
            } => format!(
                "Inconsistent number of columns in row {}: expected {}, got {}",
                row, expected_n_cols, actual_n_cols
            ),
            ParseError::NotEnoughExitTiles { n_starts, n_exits } => {
                format!("Not enough exit tiles: {n_starts} starts, {n_exits} exits")
            }
            ParseError::EmptyWorld => "Empty world: no tiles".into(),
            ParseError::NoAgents => "No agents in the world".into(),

            ParseError::InvalidTile {
                tile_str,
                line,
                col,
            } => format!("Invalid tile '{tile_str}' at position ({line}, {col})"),
            ParseError::InvalidAgentId { given_agent_id } => {
                format!("Can not parse agent id: {given_agent_id}. Expected an interger >= 0.")
            }
            ParseError::InvalidLaserSourceAgentId { asked_id, n_agents } => {
                format!(
                    "Invalid laser source agent id: {asked_id}. There are only {n_agents} agents -> expected an id between 0 and {}.",
                    n_agents - 1
                )
            }
            ParseError::InvalidDirection { given, expected } => {
                format!("Invalid direction: {given}. {expected}")
            }
            ParseError::InvalidFileName { .. } | ParseError::InvalidLevel { .. } => {
                unreachable!("Already handled above")
            }
            ParseError::NotEnoughStartTiles { n_starts, n_agents } => {
                format!("Not enough start tiles: {n_starts} starts, {n_agents} agents")
            }
            ParseError::AgentWithoutStart { agent_id } => {
                format!("Agent {agent_id} has no start tile")
            }
            ParseError::InconsistentNumberOfAgents {
                toml_n_agents_field,
                actual_n_agents,
            } => format!(
                "It is explicitely specified that there are {toml_n_agents_field} agents, but there are actually {actual_n_agents} agents in the rest of the configuration."
            ),
            ParseError::InconsistentWorldStringWidth {
                toml_width,
                world_str_width,
            } => format!(
                "Inconsistent world string width: toml width is {toml_width}, world string width is {world_str_width}"
            ),
            ParseError::InconsistentWorldStringHeight {
                toml_height,
                world_str_height,
            } => format!(
                "Inconsistent world string height: toml height is {toml_height}, world string height is {world_str_height}"
            ),
            ParseError::PositionOutOfBounds { i, j } => {
                format!("Position ({i}, {j}) is out of the world's boundaries")
            }
            ParseError::MissingHeight => "Missing height in the world configuration file".into(),
            ParseError::MissingWidth => "Missing width in the world configuration file".into(),
            ParseError::UnknownTomlKey { message, .. } => message,
            ParseError::NotV2 => panic!("NotV2 exception should not be raised here"),
        };
        Err(ParsingError::new_err(msg))
    }
}

impl From<ParseError> for PyErr {
    fn from(err: ParseError) -> Self {
        err.into()
    }
}
