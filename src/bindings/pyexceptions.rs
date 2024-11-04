use pyo3::{
    create_exception,
    exceptions::{self, PyValueError},
    PyErr,
};

use crate::{ParseError, RuntimeWorldError};

create_exception!(
    lle.exceptions,
    InvalidWorldStateError,
    PyValueError,
    "Raised when the state of the world is invalid."
);

create_exception!(
    lle.exceptions,
    InvalidActionError,
    PyValueError,
    "Raised when the action taken by an agent is invalid or when the number of actions provided is different from the number of agents."
);

create_exception!(
    lle.exceptions,
    ParsingError,
    PyValueError,
    "Raised when there is a problem while parsing a world string."
);

create_exception!(
    lle.exceptions,
    InvalidLevelError,
    PyValueError,
    "Raised when the level asked does not exist."
);

pub fn parse_error_to_exception(error: ParseError) -> PyErr {
    if let ParseError::InvalidFileName { file_name } = error {
        return exceptions::PyFileNotFoundError::new_err(file_name);
    }
    if let ParseError::InvalidLevel { asked, min, max } = error {
        return InvalidLevelError::new_err(format!(
            "Invalid level: {asked}. Expected a level between {min} and {max}.",
            asked = asked,
            min = min,
            max = max
        ));
    }
    let msg = match error {
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
                "Invalid laser source agent id: {asked_id}. There are only {n_agents} agents -> expected an id between 0 and {}.", n_agents -1
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
        },
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
        ParseError::PositionOutOfBounds { i, j } => format!("Position ({i}, {j}) is out of the world's boundaries"),
        ParseError::MissingHeight => "Missing height in the world configuration file".into(),
        ParseError::MissingWidth => "Missing width in the world configuration file".into(),
        ParseError::NotV2 => panic!("NotV2 exception should not be raised here"),
    };
    ParsingError::new_err(msg)
}

pub fn runtime_error_to_pyexception(error: RuntimeWorldError) -> PyErr {
    if let RuntimeWorldError::InvalidAction {
        agent_id,
        available,
        taken,
    } = error
    {
        return InvalidActionError::new_err(format!(
            "Invalid action for agent {agent_id}: available actions: {available:?}, taken action: {taken}",
        ));
    }

    match error {
        RuntimeWorldError::InvalidNumberOfActions { given, expected } => {
            exceptions::PyValueError::new_err(format!(
                "Invalid number of actions: given {given}, expected {expected}",
            ))
        }
        RuntimeWorldError::OutOfWorldPosition { position } => exceptions::PyValueError::new_err(
            format!("Position {position:?} is out of the world's boundaries",),
        ),
        RuntimeWorldError::InvalidNumberOfAgents { given, expected } => {
            InvalidWorldStateError::new_err(format!(
                "Invalid number of agents: given {given}, expected {expected}",
            ))
        }
        RuntimeWorldError::InvalidNumberOfGems { given, expected } => {
            InvalidWorldStateError::new_err(format!(
                "Invalid number of gems: given {given}, expected {expected}",
            ))
        }
        RuntimeWorldError::InvalidAgentPosition { position, reason } => {
            InvalidWorldStateError::new_err(format!(
                "Invalid agent position {position:?}: {reason}",
            ))
        }
        RuntimeWorldError::InvalidAction { .. } => panic!("Already handled above"),
        RuntimeWorldError::InvalidWorldState { reason, state } => InvalidWorldStateError::new_err(
            format!("Invalid world state: {reason}. Wrong state: {state:?}"),
        ),
        RuntimeWorldError::TileNotWalkable => {
            InvalidWorldStateError::new_err("An agent tried to walk on a non-walkable tile.")
        }
        RuntimeWorldError::MutexPoisoned => {
            panic!("Mutex poisoned ! Check your code for deadlocks or exceptions.")
        }
    }
}
