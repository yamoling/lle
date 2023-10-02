use crate::tiles::Direction;
use pyo3::prelude::*;

#[pyclass(name = "Direction")]
#[derive(Clone)]
pub struct PyDirection {
    direction: Direction,
}

impl PyDirection {
    pub fn new(direction: Direction) -> Self {
        Self { direction }
    }
}

#[pymethods]
impl PyDirection {
    #[classattr]
    const NORTH: Self = Self {
        direction: Direction::North,
    };
    #[classattr]
    const EAST: Self = Self {
        direction: Direction::East,
    };
    #[classattr]
    const SOUTH: Self = Self {
        direction: Direction::South,
    };
    #[classattr]
    const WEST: Self = Self {
        direction: Direction::West,
    };

    #[getter]
    pub fn name(&self) -> String {
        self.direction.to_string()
    }
}
