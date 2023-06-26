use pyo3::prelude::*;

use crate::Action;

#[pyclass(unsendable, name = "Action")]
#[derive(Clone, PartialEq, PartialOrd, Eq, Ord)]
pub struct PyAction {
    pub action: Action,
}

#[pymethods]
impl PyAction {
    #[classattr]
    const N: usize = 5;

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

    #[getter]
    fn delta(&self) -> (i32, i32) {
        self.action.delta()
    }

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

    #[getter]
    fn name(&self) -> String {
        match self.action {
            Action::North => "NORTH",
            Action::South => "SOUTH",
            Action::East => "EAST",
            Action::West => "WEST",
            Action::Stay => "STAY",
        }.into()
    }
}
