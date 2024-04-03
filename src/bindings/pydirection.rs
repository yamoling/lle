use crate::tiles::Direction;
use pyo3::{prelude::*, pyclass::CompareOp};

#[pyclass(name = "Direction", module = "lle")]
#[derive(Clone, Debug)]
pub struct PyDirection {
    direction: Direction,
}

impl From<Direction> for PyDirection {
    fn from(direction: Direction) -> Self {
        Self { direction }
    }
}

#[pymethods]
impl PyDirection {
    #[new]
    /// This constructor is required for pickling but should not be used for any other purpose.
    pub fn new() -> Self {
        Self {
            direction: Direction::North,
        }
    }

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

    fn __richcmp__(&self, other: &Self, op: CompareOp) -> PyResult<bool> {
        match op {
            CompareOp::Eq => Ok(self.direction == other.direction),
            CompareOp::Ne => Ok(self.direction != other.direction),
            _ => Err(pyo3::exceptions::PyTypeError::new_err(
                "Invalid comparison operator for Direction.",
            )),
        }
    }

    fn __str__(&self) -> String {
        self.direction.to_string()
    }

    fn __repr__(&self) -> String {
        self.direction.to_string()
    }

    #[getter]
    pub fn name(&self) -> String {
        self.direction.to_string()
    }

    pub fn __getstate__(&self) -> String {
        match self.direction {
            Direction::North => "N",
            Direction::East => "E",
            Direction::South => "S",
            Direction::West => "W",
        }
        .to_string()
    }

    pub fn __setstate__(&mut self, state: String) {
        self.direction = Direction::try_from(state.as_str()).unwrap();
    }
}
