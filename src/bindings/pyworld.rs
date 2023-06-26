use pyo3::{exceptions, prelude::*};

use crate::{Renderer, World, WorldError};

use super::pyaction::PyAction;

#[pyclass(unsendable, name = "World")]
#[derive(Clone)]
pub struct PyWorld {
    world: World,
    renderer: Renderer,
}

#[pymethods]
impl PyWorld {
    #[new]
    pub fn new(level: String) -> PyResult<Self> {
        match World::from_file(&level) {
            Ok(world) => {
                let renderer =
                    Renderer::new(world.width() as u32, world.height() as u32, world.tiles());
                Ok(PyWorld { world, renderer })
            }
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
    /// Return the rendering dimsions (width, height)
    pub fn image_dimentions(&self) -> (u32, u32) {
        (self.renderer.pixel_width(), self.renderer.pixel_height())
    }

    #[getter]
    pub fn width(&self) -> usize {
        self.world.width()
    }

    #[getter]
    pub fn height(&self) -> usize {
        self.world.height()
    }

    pub fn step(&mut self, actions: Vec<PyAction>) -> i32 {
        let actions = actions.into_iter().map(|a| a.action).collect();
        self.world.step(&actions)
    }

    pub fn reset(&mut self) {
        self.world.reset()
    }

    pub fn available_actions(&self) -> Vec<Vec<PyAction>> {
        self.world
            .available_actions()
            .into_iter()
            .map(|a| a.into_iter().map(|a| PyAction { action: a }).collect())
            .collect()
    }

    #[getter]
    pub fn done(&self) -> bool {
        self.world.done()
    }

    pub fn get_image(&mut self) -> Vec<u8> {
        let img = self.renderer.update(self.world.tiles());
        img.to_vec()
    }
}
