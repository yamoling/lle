use pyo3::{exceptions, prelude::*};

use crate::{Action, World, WorldError};

#[pyclass(unsendable)]
#[derive(Clone)]
pub struct PyWorld {
    world: World,
}

#[pymethods]
impl PyWorld {
    #[new]
    pub fn new(level: String) -> PyResult<Self> {
        match World::from_file(&level) {
            Ok(w) => Ok(PyWorld { world: w }),
            Err(e) => match e {
                WorldError::InvalidFileName { file_name } => {
                    Err(exceptions::PyFileNotFoundError::new_err(file_name))
                }
                WorldError::AgentKilledOnStartup {
                    agent_num,
                    laser_num,
                    i,
                    j,
                } => Err(exceptions::PyRuntimeError::new_err(format!(
                    "Agent {} killed by laser {} at position ({}, {})",
                    agent_num, laser_num, i, j
                ))),
                WorldError::InconsistentDimensions {
                    expected_n_cols,
                    actual_n_cols,
                    row,
                } => Err(exceptions::PyValueError::new_err(format!(
                    "Inconsistent number of columns in row {}: expected {}, got {}",
                    row, expected_n_cols, actual_n_cols
                ))),
                WorldError::InconsistentNumberOfAgents {
                    n_start_pos,
                    n_exit_pos,
                } => Err(exceptions::PyValueError::new_err(format!(
                    "Inconsistent number of agents: {} start positions, {} exit positions",
                    n_start_pos, n_exit_pos
                ))),
                WorldError::InvalidPosition { x, y } => {
                    panic!(
                        "Unexpected error 'InvalidPosition' while building a new World: {}, {}",
                        x, y
                    )
                }
            },
        }
    }

    #[getter]
    pub fn n_agents(&self) -> usize {
        self.world.n_agents()
    }

    #[getter]
    pub fn width(&self) -> usize {
        self.world.width()
    }

    #[getter]
    pub fn height(&self) -> usize {
        self.world.height()
    }

    pub fn step(&mut self, actions: Vec<Action>) -> i32 {
        self.world.step(&actions)
    }

    pub fn reset(&mut self) {
        self.world.reset()
    }

    pub fn available_actions(&self) -> Vec<Vec<Action>> {
        self.world.available_actions()
    }

    pub fn done(&self) -> bool {
        self.world.done()
    }
}
