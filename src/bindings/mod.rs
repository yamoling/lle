use pyo3::prelude::*;

mod pyaction;
mod pyagent;
mod pydirection;
mod pyevent;
mod pytile;
mod pyworld;
mod pyworld_state;

// pub use pyworld::PyWorld;

#[pymodule]
pub fn lle(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_class::<pyevent::PyEventType>()?;
    m.add_class::<pyevent::PyWorldEvent>()?;
    m.add_class::<pyworld::PyWorld>()?;
    m.add_class::<pyworld_state::PyWorldState>()?;
    m.add_class::<pyaction::PyAction>()?;
    m.add_class::<pyagent::PyAgent>()?;
    m.add_class::<pydirection::PyDirection>()?;
    m.add_class::<pytile::PyGem>()?;
    m.add_class::<pytile::PyLaser>()?;
    m.add_class::<pytile::PyLaserSource>()?;
    m.add("__version__", crate::VERSION)?;
    Ok(())
}
