use pyo3::prelude::*;

mod pyaction;
mod pyagent;
mod pydirection;
mod pyevent;
mod pyexceptions;
mod pytile;
mod pyworld;
mod pyworld_builder;
mod pyworld_state;

#[pymodule]
pub fn lle(py: Python, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<pyevent::PyEventType>()?;
    m.add_class::<pyevent::PyWorldEvent>()?;
    m.add_class::<pyworld::PyWorld>()?;
    m.add_class::<pyworld_builder::PyWorldBuilder>()?;
    m.add_class::<pyworld_state::PyWorldState>()?;
    m.add_class::<pyaction::PyAction>()?;
    m.add_class::<pyagent::PyAgent>()?;
    m.add_class::<pydirection::PyDirection>()?;
    m.add_class::<pytile::PyGem>()?;
    m.add_class::<pytile::PyLaser>()?;
    m.add_class::<pytile::PyLaserSource>()?;
    m.add(
        "InvalidWorldStateError",
        py.get_type_bound::<pyexceptions::InvalidWorldStateError>(),
    )?;
    m.add(
        "InvalidActionError",
        py.get_type_bound::<pyexceptions::InvalidActionError>(),
    )?;
    m.add(
        "ParsingError",
        py.get_type_bound::<pyexceptions::ParsingError>(),
    )?;
    m.add(
        "InvalidLevelError",
        py.get_type_bound::<pyexceptions::InvalidLevelError>(),
    )?;
    m.add("__version__", crate::VERSION)?;
    Ok(())
}
