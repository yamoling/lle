use pyo3::{prelude::*, pyclass::CompareOp};
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pymethods};

use crate::Action;

/// An action that can be taken in the world by the agents.
#[gen_stub_pyclass]
#[pyclass(name = "Action", module = "lle")]
#[derive(Clone)]
pub struct PyAction {
    pub action: Action,
}

#[gen_stub_pymethods]
#[pymethods]
impl PyAction {
    /// The number of actions
    #[classattr]
    const N: usize = PyAction::ALL.len();

    #[classattr]
    const NORTH: Self = Self {
        action: Action::North,
    };

    #[classattr]
    const SOUTH: Self = Self {
        action: Action::South,
    };

    #[classattr]
    const EAST: Self = Self {
        action: Action::East,
    };

    #[classattr]
    const WEST: Self = Self {
        action: Action::West,
    };

    #[classattr]
    const STAY: Self = Self {
        action: Action::Stay,
    };

    /// The (i, j) position delta in coordinates for this action.
    #[getter]
    fn delta(&self) -> (i32, i32) {
        self.action.delta()
    }

    /// Ordered list of all actions
    #[classattr]
    const ALL: [Self; 5] = [
        Self {
            action: Action::North,
        },
        Self {
            action: Action::South,
        },
        Self {
            action: Action::East,
        },
        Self {
            action: Action::West,
        },
        Self {
            action: Action::Stay,
        },
    ];

    #[new]
    fn new(value: u32) -> PyResult<Self> {
        match value {
            0 => Ok(Self {
                action: Action::North,
            }),
            1 => Ok(Self {
                action: Action::South,
            }),
            2 => Ok(Self {
                action: Action::East,
            }),
            3 => Ok(Self {
                action: Action::West,
            }),
            4 => Ok(Self {
                action: Action::Stay,
            }),
            _ => Err(pyo3::exceptions::PyValueError::new_err(format!(
                "Invalid action value: {value}. Valid values for actions are between 0 and 4."
            ))),
        }
    }

    fn __richcmp__(&self, other: &Self, op: CompareOp) -> PyResult<bool> {
        match op {
            CompareOp::Eq => Ok(self.action == other.action),
            CompareOp::Ne => Ok(self.action != other.action),
            _ => Err(pyo3::exceptions::PyTypeError::new_err(
                "Invalid comparison operator for Action.",
            )),
        }
    }

    fn __str__(&self) -> String {
        self.action.to_string()
    }

    fn __repr__(&self) -> String {
        self.action.to_string()
    }

    /// The integer value of this action.
    #[getter]
    fn value(&self) -> u32 {
        match self.action {
            Action::North => 0,
            Action::South => 1,
            Action::East => 2,
            Action::West => 3,
            Action::Stay => 4,
        }
    }

    /// The string name of this action.
    #[getter]
    fn name(&self) -> String {
        self.action.to_string()
    }

    /// The opposite action of this action.
    /// Note: STAY is its own opposite.
    fn opposite(&self) -> Self {
        Self {
            action: self.action.opposite().into(),
        }
    }
}
