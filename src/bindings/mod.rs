use pyo3::prelude::*;

#[pymodule]
pub fn lle(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_class::<crate::World>()?;
    m.add_class::<crate::Action>()?;
    Ok(())
}
