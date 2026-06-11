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
        use super::utils::RegisterSubmodules;

        let py = m.py();
        // Workaround for to be able to write `from lle.tiles import X`.
        // See https://github.com/PyO3/pyo3/issues/759
        m.register_submodules("lle")?;
        m.add("__version__", crate::VERSION)?;

        // `lle.solver` is a regular Python package (`python/lle/solver/__init__.py`), so unlike
        // the other submodules we must not register a native module at `lle.solver`.
        // Instead, we register `lle.solver.constraints` directly in sys.modules so the
        //  Python package finds it already present when it does `from .constraints import ...`.
        let sys_modules = py.import("sys")?.getattr("modules")?;
        let constraints = PyModule::new(py, "constraints")?;
        constraints.add_class::<super::solver::PyClauseGenerator>()?;
        constraints.add_class::<super::solver::PySolveMode>()?;
        sys_modules.set_item("lle.solver.constraints", &constraints)
    }
}

define_stub_info_gatherer!(stub_info);
