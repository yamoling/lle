use std::{error::Error, fmt::Display};

use crate::{agent::AgentId, Action};

#[derive(Debug)]
pub enum WorldError {
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
    NotEnoughExitTiles {
        n_starts: usize,
        n_exits: usize,
    },
    InconsistentDimensions {
        expected_n_cols: usize,
        actual_n_cols: usize,
        row: usize,
    },
    AgentKilledOnStartup {
        agent_num: u32,
        laser_num: u32,
        i: usize,
        j: usize,
    },
    InvalidPosition {
        x: i32,
        y: i32,
    },
}

impl Display for WorldError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

impl Error for WorldError {}

#[derive(Debug)]
pub enum RuntimeWorldError {
    InvalidAction {
        agent_id: AgentId,
        available: Vec<Action>,
        taken: Action,
    },
}

impl Display for RuntimeWorldError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

impl Error for RuntimeWorldError {}
