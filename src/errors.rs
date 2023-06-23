use std::{error::Error, fmt::Display};

#[derive(Debug)]
pub enum WorldError {
    InvalidFileName {
        file_name: String,
    },
    InconsistentNumberOfAgents {
        n_start_pos: usize,
        n_exit_pos: usize,
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
