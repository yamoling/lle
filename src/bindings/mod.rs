use pyo3::prelude::*;
use pyo3_stub_gen::define_stub_info_gatherer;

mod pyagent;
mod pyexceptions;
mod solver;
mod tiles;
mod utils;
mod world;

pub use pyexceptions::{
    InvalidActionError, InvalidLevelError, InvalidWorldStateError, ParsingError,
};
pub use solver::{PyClauseGenerator, PySolveMode};
pub use tiles::{PyLaser, PyLaserSource};
pub use world::{PyAction, PyEventType, PyPosition, PyWorld, PyWorldEvent, PyWorldState};

#[pymodule]
mod lle {
    use pyo3::prelude::*;

    #[pymodule]
    mod tiles {
        #[pymodule_export]
        use super::super::tiles::PyDirection;
        #[pymodule_export]
        use super::super::tiles::PyGem;
        #[pymodule_export]
        use super::super::tiles::PyLaser;
        #[pymodule_export]
        use super::super::tiles::PyLaserSource;
    }

    #[pymodule]
    mod world {
        use pyo3::prelude::*;

        #[pymodule]
        #[pyo3(name = "rendering")]
        mod rendering_module {
            use pyo3::prelude::*;
            use pyo3_stub_gen::module_variable;

            module_variable!("lle.world.rendering", "TILE_SIZE", u32);

            #[pymodule_init]
            fn init(m: &Bound<'_, PyModule>) -> PyResult<()> {
                m.add("TILE_SIZE", crate::rendering::TILE_SIZE)
            }
        }

        #[pymodule_export]
        use self::rendering_module as rendering;
        #[pymodule_export]
        use super::super::world::PyAction;
        #[pymodule_export]
        use super::super::world::PyEventType;
        #[pymodule_export]
        use super::super::world::PyWorld;
        #[pymodule_export]
        use super::super::world::PyWorldEvent;
        #[pymodule_export]
        use super::super::world::PyWorldState;
    }

    #[pymodule]
    mod agent {
        #[pymodule_export]
        use super::super::pyagent::PyAgent;
    }

    #[pymodule]
    mod exceptions {
        #[pymodule_export]
        use super::super::pyexceptions::InvalidActionError;
        #[pymodule_export]
        use super::super::pyexceptions::InvalidLevelError;
        #[pymodule_export]
        use super::super::pyexceptions::InvalidWorldStateError;
        #[pymodule_export]
        use super::super::pyexceptions::ParsingError;
        #[pymodule_export]
        use super::super::pyexceptions::SolverError;
    }

    #[pymodule_init]
    fn init(m: &Bound<'_, PyModule>) -> PyResult<()> {
        let py = m.py();
        // Workaround for to be able to write `from lle.tiles import X`.
        // See https://github.com/PyO3/pyo3/issues/759
        super::utils::register_submodules(m, "lle")?;
        m.add("__version__", crate::VERSION)?;

        // `lle.solver` is a regular Python package (`python/lle/solver/__init__.py`), so unlike
        // the other submodules we must not register a native module at `lle.solver`.
        // Instead, we register `lle.solver.clauses` directly in sys.modules so the
        //  Python package finds it already present when it does `from .clauses import ...`.
        let sys_modules = py.import("sys")?.getattr("modules")?;
        let clauses = PyModule::new(py, "clauses")?;
        clauses.add_class::<super::solver::PyClauseGenerator>()?;
        clauses.add_class::<super::solver::PySolveMode>()?;
        sys_modules.set_item("lle.solver.clauses", &clauses)
    }
}

define_stub_info_gatherer!(stub_info);
