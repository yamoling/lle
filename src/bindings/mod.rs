use pyo3::prelude::*;
use pyo3_stub_gen::define_stub_info_gatherer;

mod pyagent;
mod pyexceptions;
mod tiles;
mod world;

pub use pyexceptions::{
    InvalidActionError, InvalidLevelError, InvalidWorldStateError, ParsingError,
};
pub use tiles::{PyLaser, PyLaserSource};
pub use world::{PyAction, PyEventType, PyPosition, PyWorld, PyWorldEvent, PyWorldState};

fn make_tiles_submodule<'py>(py: Python<'py>) -> PyResult<Bound<'py, PyModule>> {
    let tiles = PyModule::new(py, "tiles")?;
    tiles.add_class::<tiles::PyDirection>()?;
    tiles.add_class::<tiles::PyGem>()?;
    tiles.add_class::<tiles::PyLaser>()?;
    tiles.add_class::<tiles::PyLaserSource>()?;
    Ok(tiles)
}

fn make_world_submodule<'py>(py: Python<'py>) -> PyResult<Bound<'py, PyModule>> {
    let world = PyModule::new(py, "world")?;
    world.add_class::<world::PyWorld>()?;
    world.add_class::<world::PyWorldState>()?;
    world.add_class::<world::PyEventType>()?;
    world.add_class::<world::PyWorldEvent>()?;
    world.add_class::<world::PyAction>()?;
    Ok(world)
}

fn make_exceptions_submodule<'py>(py: Python<'py>) -> PyResult<Bound<'py, PyModule>> {
    let exceptions = PyModule::new(py, "exceptions")?;
    exceptions.add(
        "InvalidWorldStateError",
        py.get_type::<pyexceptions::InvalidWorldStateError>(),
    )?;
    exceptions.add(
        "InvalidActionError",
        py.get_type::<pyexceptions::InvalidActionError>(),
    )?;
    exceptions.add("ParsingError", py.get_type::<pyexceptions::ParsingError>())?;
    exceptions.add(
        "InvalidLevelError",
        py.get_type::<pyexceptions::InvalidLevelError>(),
    )?;
    Ok(exceptions)
}

fn add_submodule<'py>(
    py: Python<'py>,
    module: &Bound<'_, PyModule>,
    submodule: Bound<'_, PyModule>,
) -> PyResult<()> {
    let submodule_name: String = submodule
        .name()
        .unwrap()
        .to_string()
        .split('.')
        .last()
        .unwrap()
        .into();
    let module_name: String = module
        .name()
        .unwrap()
        .to_string()
        .split('.')
        .last()
        .unwrap()
        .into();
    // We use "m.add()" instead of "m.add_submodule()" to avoid import problems.
    // With "m.add_submodule()", it is not possible to do `from lle.tiles import X`.
    // cf: https://github.com/PyO3/pyo3/issues/759
    module.add(&submodule_name, &submodule)?;
    let total_path = format!("{module_name}.{submodule_name}");
    py.import("sys")?
        .getattr("modules")?
        .set_item(total_path, &submodule)
}

#[pymodule]
fn lle(py: Python, m: &Bound<'_, PyModule>) -> PyResult<()> {
    let tiles = make_tiles_submodule(py)?;
    let world = make_world_submodule(py)?;
    let exceptions = make_exceptions_submodule(py)?;

    add_submodule(py, m, tiles)?;
    add_submodule(py, m, world)?;
    add_submodule(py, m, exceptions)?;

    let agent = PyModule::new(py, "agent")?;
    agent.add_class::<pyagent::PyAgent>()?;
    add_submodule(py, m, agent)?;

    m.add("__version__", crate::VERSION)?;
    Ok(())
}

define_stub_info_gatherer!(stub_info);
