use crate::tiles::Direction;
use pyo3::{prelude::*, types::PyTuple};
use pyo3_stub_gen::derive::{gen_stub_pyclass_enum, gen_stub_pymethods};

#[gen_stub_pyclass_enum]
#[pyclass(name = "Direction", module = "lle.tiles", eq)]
#[derive(Clone, Debug, PartialEq)]
pub enum PyDirection {
    #[pyo3(name = "NORTH")]
    North,
    #[pyo3(name = "EAST")]
    East,
    #[pyo3(name = "SOUTH")]
    South,
    #[pyo3(name = "WEST")]
    West,
}

impl TryFrom<&str> for PyDirection {
    type Error = &'static str;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "N" => Ok(Self::North),
            "E" => Ok(Self::East),
            "S" => Ok(Self::South),
            "W" => Ok(Self::West),
            _ => Err("Invalid direction string."),
        }
    }
}

impl From<Direction> for PyDirection {
    fn from(direction: Direction) -> Self {
        match direction {
            Direction::North => Self::North,
            Direction::East => Self::East,
            Direction::South => Self::South,
            Direction::West => Self::West,
        }
    }
}

impl Into<Direction> for &PyDirection {
    fn into(self) -> Direction {
        match self {
            PyDirection::North => Direction::North,
            PyDirection::East => Direction::East,
            PyDirection::South => Direction::South,
            PyDirection::West => Direction::West,
        }
    }
}

impl Into<&str> for PyDirection {
    fn into(self) -> &'static str {
        match self {
            PyDirection::North => "N",
            PyDirection::East => "E",
            PyDirection::South => "S",
            PyDirection::West => "W",
        }
    }
}

#[gen_stub_pymethods]
#[pymethods]
impl PyDirection {
    #[new]
    /// This constructor is required for pickling but should not be used for any other purpose.
    pub fn new(direction: String) -> PyResult<Self> {
        let direction = match PyDirection::try_from(direction.as_str()) {
            Ok(direction) => direction,
            Err(e) => return Err(pyo3::exceptions::PyValueError::new_err(e.to_string())),
        };
        Ok(direction)
    }

    /// Creates a `Direction` from a string representation.
    ///
    /// Args:
    ///    direction (Literal["N", "E", "S", "W"]): The string direction to create.
    ///
    /// Returns:
    ///   The corresponding `Direction` object.
    ///
    /// Raises:
    ///   ValueError: If the string is not a valid cardinal direction.
    ///
    #[staticmethod]
    fn from_str(direction: String) -> PyResult<Self> {
        match PyDirection::try_from(direction.as_str()) {
            Ok(direction) => Ok(direction),
            Err(_) => Err(pyo3::exceptions::PyValueError::new_err(
                "Invalid direction string.",
            )),
        }
    }

    /// The delta of this direction (di, dj).
    fn delta(&self) -> (i32, i32) {
        let d: Direction = self.into();
        d.delta()
    }

    /// The opposite of this direction.
    fn opposite(&self) -> PyDirection {
        let d: Direction = self.into();
        d.opposite().into()
    }

    #[getter]
    fn is_horizontal(&self) -> bool {
        match self {
            Self::North | Self::South => false,
            Self::East | Self::West => true,
        }
    }

    #[getter]
    fn is_vertical(&self) -> bool {
        !self.is_horizontal()
    }

    fn __repr__(&self) -> String {
        self.name()
    }

    #[getter]
    pub fn name(&self) -> String {
        match self {
            Self::North => "N",
            Self::East => "E",
            Self::South => "S",
            Self::West => "W",
        }
        .to_string()
    }

    pub fn __getstate__(&self) -> String {
        self.name()
    }

    /// This method is called to instantiate the object before deserialisation.
    /// It required "default arguments" to be provided to the __new__ method
    /// before replacing them by the actual values in __setstate__.
    pub fn __getnewargs__(&self, py: Python) -> PyObject {
        PyTuple::new_bound(py, vec![String::from("N")].iter()).into()
    }

    pub fn __setstate__(&mut self, state: String) {
        *self = PyDirection::try_from(state.as_str()).unwrap();
    }
}
