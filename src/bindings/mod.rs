use pyo3::prelude::*;
use pyo3_stub_gen::define_stub_info_gatherer;

mod pyaction;
mod pyagent;
mod pydirection;
mod pyevent;
mod pyexceptions;
mod pyposition;
mod pytile;
mod pyworld;
// mod pyworld_builder;
mod pyworld_state;

pub use pyaction::PyAction;
pub use pytile::{PyLaser, PyLaserSource};
pub use pyworld::PyWorld;
// pub use pyworld_builder::PyWorldBuilder;
pub use pyworld_state::PyWorldState;

#[pymodule]
pub fn lle(py: Python, m: &Bound<'_, PyModule>) -> PyResult<()> {
    let tiles = PyModule::new_bound(py, "lle.tiles")?;
    tiles.add_class::<pytile::PyGem>()?;
    tiles.add_class::<pytile::PyLaser>()?;
    tiles.add_class::<pytile::PyLaserSource>()?;
    tiles.add_class::<pydirection::PyDirection>()?;
    // We use "m.add()" instead of "m.add_submodule()" to avoid import problems.
    // With "m.add_submodule()", it is not possible to do `from lle.tiles import X`.
    // cf: https://github.com/PyO3/pyo3/issues/759
    m.add("tiles", &tiles)?;
    py.import_bound("sys")?
        .getattr("modules")?
        .set_item("lle.tiles", &tiles)?;

    m.add_class::<pyevent::PyEventType>()?;
    m.add_class::<pyevent::PyWorldEvent>()?;
    // m.add_class::<pyworld_builder::PyWorldBuilder>()?;
    m.add_class::<pyaction::PyAction>()?;
    m.add_class::<pyagent::PyAgent>()?;
    m.add_class::<pyworld::PyWorld>()?;
    m.add_class::<pyworld_state::PyWorldState>()?;

    let exceptions = PyModule::new_bound(py, "lle.exceptions")?;
    // exceptions.add_class::<pyexceptions::InvalidWorldStateError>()?;
    exceptions.add(
        "InvalidWorldStateError",
        py.get_type_bound::<pyexceptions::InvalidWorldStateError>(),
    )?;
    exceptions.add(
        "InvalidActionError",
        py.get_type_bound::<pyexceptions::InvalidActionError>(),
    )?;
    exceptions.add(
        "ParsingError",
        py.get_type_bound::<pyexceptions::ParsingError>(),
    )?;
    exceptions.add(
        "InvalidLevelError",
        py.get_type_bound::<pyexceptions::InvalidLevelError>(),
    )?;
    m.add("exceptions", &exceptions)?;
    py.import_bound("sys")?
        .getattr("modules")?
        .set_item("lle.exceptions", &exceptions)?;
    m.add("__version__", crate::VERSION)?;
    Ok(())
}

define_stub_info_gatherer!(stub_info);
