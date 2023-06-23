use pyo3::prelude::*;

use crate::Action;

#[pyclass(unsendable)]
pub struct PyAction {
    pub action: Action,
}

#[pymethods]
impl PyAction {
    #[staticmethod]
    pub fn from_str(s: &str) -> Self {
        Self {
            action: Action::from(s),
        }
    }

    #[getter]
    pub fn delta(&self) -> (i32, i32) {
        self.action.delta()
    }
}
