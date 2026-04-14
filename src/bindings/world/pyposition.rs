use crate::Position;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::pyclass::CompareOp;
use pyo3_stub_gen::PyStubType;
use pyo3_stub_gen::TypeInfo;
use std::hash::{Hash, Hasher};
/// A position which can be used to represent the location in the world.
/// it can be constructed from a 2-tuple (i, j) or a 3-tuple (i, j, k), where k defaults to 0 if not provided.
#[pyclass(name = "Position", module = "lle.world", skip_from_py_object)]
#[derive(Clone, Hash, Debug, Copy, PartialEq, PartialOrd)]
pub struct PyPosition(pub usize, pub usize, pub usize);

impl FromPyObject<'_, '_> for PyPosition {
    type Error = PyErr;

    /// Convert a Python object in the form of a 2-tuple (i, j) or a 3-tuple (i, j, k) into a PyPosition.
    /// If a 2-tuple is provided, k will default to 0. If the object cannot be converted, a ValueError is raised.
    fn extract(obj: pyo3::Borrowed<'_, '_, PyAny>) -> Result<Self, Self::Error> {
        if let Ok(pos) = obj.cast::<PyPosition>() {
            let pos = pos.borrow();
            return Ok(PyPosition(pos.0, pos.1, pos.2));
        }
        // Try 3-tuple first
        if let Ok((i, j, k)) = obj.extract::<(usize, usize, usize)>() {
            return Ok(PyPosition(i, j, k));
        }
        // Try 2-tuple, defaulting k=0
        if let Ok((i, j)) = obj.extract::<(usize, usize)>() {
            return Ok(PyPosition(i, j, 0));
        }
        Err(PyValueError::new_err(
            "Position must be a tuple of 2 or 3 non-negative integers",
        ))
    }
}

impl PyStubType for PyPosition {
    fn type_output() -> TypeInfo {
        TypeInfo::builtin("tuple[int, int, int]")
    }

    fn type_input() -> TypeInfo {
        TypeInfo::builtin("tuple[int, int] | tuple[int, int, int]")
    }
}

/// An iterator for PyPosition for unpacking the position into its components (i, j, k).
#[pyclass]
pub struct PyPositionIter {
    values: [usize; 3],
    index: usize,
}

#[pymethods]
impl PyPositionIter {
    fn __iter__(slf: PyRef<Self>) -> PyRef<Self> {
        slf
    }

    fn __next__(&mut self) -> Option<usize> {
        if self.index < 3 {
            let val = self.values[self.index];
            self.index += 1;
            Some(val)
        } else {
            None
        }
    }
}
#[pymethods]
impl PyPosition {
    #[new]
    fn new(i: usize, j: usize, k: usize) -> Self {
        PyPosition(i, j, k)
    }

    fn __iter__(&self) -> PyPositionIter {
        PyPositionIter {
            values: [self.0, self.1, self.2],
            index: 0,
        }
    }

    fn __len__(&self) -> usize {
        3
    }

    fn __getitem__(&self, idx: usize) -> PyResult<usize> {
        match idx {
            0 => Ok(self.0),
            1 => Ok(self.1),
            2 => Ok(self.2),
            _ => Err(PyValueError::new_err("Index out of range for Position")),
        }
    }

    fn __str__(&self) -> String {
        format!("({}, {}, {})", self.0, self.1, self.2)
    }

    fn __repr__(&self) -> String {
        format!("PyPosition({}, {}, {})", self.0, self.1, self.2)
    }

    fn __richcmp__(&self, other: &Bound<'_, PyAny>, op: CompareOp) -> PyResult<bool> {
        // check if other is a PyPosition
        let otherpos = match other.extract::<PyPosition>() {
            Ok(pos) => pos,
            Err(_) => {
                return Err(PyValueError::new_err(format!(
                    "Cannot compare PyPosition with object of type {}",
                    other.get_type().name()?
                )));
            }
        };
        let result = match op {
            CompareOp::Eq => self == &otherpos,
            CompareOp::Ne => self != &otherpos,
            CompareOp::Lt => self < &otherpos,
            CompareOp::Le => self <= &otherpos,
            CompareOp::Gt => self > &otherpos,
            CompareOp::Ge => self >= &otherpos,
        };

        Ok(result)
    }
    fn __hash__(&self) -> u64 {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        self.hash(&mut hasher);
        hasher.finish()
    }

    fn __getstate__(&self) -> (usize, usize, usize) {
        (self.0, self.1, self.2)
    }

    fn __setstate__(&mut self, state: (usize, usize, usize)) {
        self.0 = state.0;
        self.1 = state.1;
        self.2 = state.2;
    }

    fn __getnewargs__(&self) -> (usize, usize, usize) {
        (self.0, self.1, self.2)
    }
}

impl From<Position> for PyPosition {
    fn from(pos: Position) -> Self {
        let (i, j, k) = pos.as_ijk();
        PyPosition(i, j, k)
    }
}

impl From<&Position> for PyPosition {
    fn from(pos: &Position) -> Self {
        let (i, j, k) = pos.as_ijk();
        PyPosition(i, j, k)
    }
}

impl From<PyPosition> for Position {
    fn from(pos: PyPosition) -> Self {
        Position {
            i: pos.0,
            j: pos.1,
            k: pos.2,
        }
    }
}

impl From<&PyPosition> for Position {
    fn from(pos: &PyPosition) -> Self {
        Position {
            i: pos.0,
            j: pos.1,
            k: pos.2,
        }
    }
}
