use std::ops::Deref;

use pyo3::prelude::*;
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pymethods};

use crate::Position;

#[gen_stub_pyclass]
#[pyclass(name = "Position", module = "lle")]
#[derive(Debug, Clone, PartialEq, PartialOrd, Eq, Ord, Hash)]
pub struct PyPosition {
    #[pyo3(get, set)]
    pub i: usize,
    #[pyo3(get, set)]
    pub j: usize,
}

impl From<(usize, usize)> for PyPosition {
    fn from(pos: (usize, usize)) -> Self {
        PyPosition { i: pos.0, j: pos.1 }
    }
}

impl From<Position> for PyPosition {
    fn from(pos: Position) -> Self {
        PyPosition { i: pos.i, j: pos.j }
    }
}

impl Into<Position> for PyPosition {
    fn into(self) -> Position {
        Position {
            i: self.i,
            j: self.j,
        }
    }
}
