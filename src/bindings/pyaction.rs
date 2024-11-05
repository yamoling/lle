use pyo3::prelude::*;
use pyo3_stub_gen::derive::{gen_stub_pyclass_enum, gen_stub_pymethods};

use crate::Action;

/// An action that can be taken in the world by the agents.
#[gen_stub_pyclass_enum]
#[pyclass(name = "Action", module = "lle", eq, eq_int)]
#[derive(Clone, Debug, PartialEq)]
pub enum PyAction {
    #[pyo3(name = "NORTH")]
    North = 0,
    #[pyo3(name = "SOUTH")]
    South = 1,
    #[pyo3(name = "EAST")]
    East = 2,
    #[pyo3(name = "WEST")]
    West = 3,
    #[pyo3(name = "STAY")]
    Stay = 4,
}

#[gen_stub_pymethods]
#[pymethods]
impl PyAction {
    /// Ordered list of all actions
    #[classattr]
    const ALL: [Self; 5] = [Self::North, Self::South, Self::East, Self::West, Self::Stay];

    /// The number of actions
    #[classattr]
    const N: usize = PyAction::ALL.len();

    /// The (i, j) position delta in coordinates for this action.
    #[getter]
    fn delta(&self) -> (i32, i32) {
        let action: Action = self.into();
        action.delta()
    }

    #[new]
    fn new(value: u32) -> PyResult<Self> {
        match value {
            0 => Ok(Self::North),
            1 => Ok(Self::South),
            2 => Ok(Self::East),
            3 => Ok(Self::West),
            4 => Ok(Self::Stay),
            _ => Err(pyo3::exceptions::PyValueError::new_err(format!(
                "Invalid action value: {value}. Valid values for actions are between 0 and 4."
            ))),
        }
    }

    fn __repr__(&self) -> String {
        format!("{self:?}")
    }

    /// The integer value of this action.
    #[getter]
    fn value(&self) -> u32 {
        match self {
            Self::North => 0,
            Self::South => 1,
            Self::East => 2,
            Self::West => 3,
            Self::Stay => 4,
        }
    }

    /// The string name of this action.
    #[getter]
    fn name(&self) -> String {
        match self {
            Self::North => "NORTH".to_string(),
            Self::South => "SOUTH".to_string(),
            Self::East => "EAST".to_string(),
            Self::West => "WEST".to_string(),
            Self::Stay => "STAY".to_string(),
        }
    }

    /// The opposite action of this action.
    /// Note: STAY is its own opposite.
    fn opposite(&self) -> Self {
        let action: Action = self.into();
        action.opposite().into()
    }
}

impl Into<Action> for &PyAction {
    fn into(self) -> Action {
        match self {
            PyAction::North => Action::North,
            PyAction::South => Action::South,
            PyAction::East => Action::East,
            PyAction::West => Action::West,
            PyAction::Stay => Action::Stay,
        }
    }
}

impl Into<Action> for PyAction {
    fn into(self) -> Action {
        match self {
            PyAction::North => Action::North,
            PyAction::South => Action::South,
            PyAction::East => Action::East,
            PyAction::West => Action::West,
            PyAction::Stay => Action::Stay,
        }
    }
}

impl From<&Action> for PyAction {
    fn from(action: &Action) -> Self {
        match action {
            Action::North => PyAction::North,
            Action::South => PyAction::South,
            Action::East => PyAction::East,
            Action::West => PyAction::West,
            Action::Stay => PyAction::Stay,
        }
    }
}

impl From<Action> for PyAction {
    fn from(action: Action) -> Self {
        match action {
            Action::North => PyAction::North,
            Action::South => PyAction::South,
            Action::East => PyAction::East,
            Action::West => PyAction::West,
            Action::Stay => PyAction::Stay,
        }
    }
}
