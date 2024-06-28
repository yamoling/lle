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

impl Into<Direction> for PyDirection {
    fn into(self) -> Direction {
        self.direction
    }
}

impl Into<&str> for PyDirection {
    fn into(self) -> &'static str {
        self.direction.into()
    }
}

#[pymethods]
impl PyDirection {
    #[new]
    /// This constructor is required for pickling but should not be used for any other purpose.
    pub fn new(direction: String) -> PyResult<Self> {
        let direction = match Direction::try_from(direction.as_str()) {
            Ok(direction) => direction,
            Err(e) => return Err(pyo3::exceptions::PyValueError::new_err(e.to_string())),
        };
        Ok(Self { direction })
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

    fn delta(&self) -> (i32, i32) {
        self.direction.delta()
    }

    fn opposite(&self) -> PyDirection {
        self.direction.opposite().into()
    }

    fn is_horizontal(&self) -> bool {
        self.direction == Direction::East || self.direction == Direction::West
    }

    fn is_vertical(&self) -> bool {
        self.direction == Direction::North || self.direction == Direction::South
    }

    #[staticmethod]
    fn from_str(direction: String) -> PyResult<Self> {
        match Direction::try_from(direction.as_str()) {
            Ok(direction) => Ok(Self { direction }),
            Err(_) => Err(pyo3::exceptions::PyValueError::new_err(
                "Invalid direction string.",
            )),
        }
    }

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

    /// This method is called to instantiate the object before deserialisation.
    /// It required "default arguments" to be provided to the __new__ method
    /// before replacing them by the actual values in __setstate__.
    pub fn __getnewargs__(&self) -> PyResult<(String,)> {
        Ok((String::from("N"),))
    }

    pub fn __setstate__(&mut self, state: String) {
        self.direction = Direction::try_from(state.as_str()).unwrap();
    }
}
