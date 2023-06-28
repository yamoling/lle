use pyo3::prelude::*;

mod pyaction;
mod pyagent;
mod pyworld;

#[pymodule]
pub fn lle(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_class::<pyworld::PyWorld>()?;
    m.add_class::<pyaction::PyAction>()?;
    // m.add_class::<crate::Action>()?;
    Ok(())
}
