use numpy::PyArray1;
use pyo3::{exceptions, prelude::*, types::PyDict};

use crate::{errors::RuntimeWorldError, Position, Renderer, World, WorldError};

use super::{
    pyaction::PyAction,
    pyagent::PyAgent,
    pytile::{PyGem, PyLaser, PyLaserSource},
};

#[pyclass(unsendable, name = "World")]
#[derive(Clone)]
pub struct PyWorld {
    world: World,
    renderer: Renderer,
}

fn error_to_exception(error: WorldError) -> PyErr {
    match error {
        WorldError::InvalidFileName { file_name } => {
            exceptions::PyFileNotFoundError::new_err(file_name)
        }
        WorldError::AgentKilledOnStartup {
            agent_num,
            laser_num,
            i,
            j,
        } => exceptions::PyRuntimeError::new_err(format!(
            "Agent {} killed by laser {} at position ({}, {})",
            agent_num, laser_num, i, j
        )),
        WorldError::InconsistentDimensions {
            expected_n_cols,
            actual_n_cols,
            row,
        } => exceptions::PyValueError::new_err(format!(
            "Inconsistent number of columns in row {}: expected {}, got {}",
            row, expected_n_cols, actual_n_cols
        )),
        WorldError::NotEnoughExitTiles { n_starts, n_exits } => exceptions::PyValueError::new_err(
            format!("Not enough exit tiles: {n_starts} starts, {n_exits} exits"),
        ),
        WorldError::EmptyWorld => exceptions::PyValueError::new_err("Empty world: no tiles"),
        WorldError::NoAgents => exceptions::PyValueError::new_err("No agents in the world"),
        WorldError::InvalidPosition { x, y } => {
            exceptions::PyIndexError::new_err(format!("Invalid position ({}, {})", x, y))
        }
        WorldError::InvalidTile {
            tile_str,
            line,
            col,
        } => exceptions::PyValueError::new_err(format!(
            "Invalid tile '{tile_str}' at position ({line}, {col})"
        )),
    }
}

#[pymethods]
impl PyWorld {
    #[new]
    pub fn new(map_str: String) -> PyResult<Self> {
        let world = match World::try_from(map_str) {
            Ok(world) => world,
            Err(e) => return Err(error_to_exception(e)),
        };
        let renderer = Renderer::new(&world);
        Ok(PyWorld { world, renderer })
    }

    #[staticmethod]
    fn from_file(filename: String) -> PyResult<Self> {
        let world = match World::from_file(&filename) {
            Ok(world) => world,
            Err(e) => return Err(error_to_exception(e)),
        };
        let renderer = Renderer::new(&world);
        Ok(PyWorld { world, renderer })
    }

    #[getter]
    fn world_string(&self) -> String {
        self.world.world_string().into()
    }

    #[getter]
    pub fn n_agents(&self) -> usize {
        self.world.n_agents()
    }

    #[getter]
    /// Return the rendering dimensions (width, height)
    pub fn image_dimensions(&self) -> (u32, u32) {
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

    #[getter]
    pub fn n_gems(&self) -> usize {
        self.world.n_gems()
    }

    #[getter]
    fn exit_rate(&self) -> f32 {
        let n_arrived: f32 = self
            .world
            .agents()
            .iter()
            .map(|a| if a.has_arrived() { 1.0f32 } else { 0.0f32 })
            .sum();
        n_arrived / (self.world.n_agents() as f32)
    }

    #[getter]
    fn gems_collected(&self) -> u32 {
        self.world.gems_collected()
    }

    #[getter]
    fn agent_positions(&self) -> Vec<Position> {
        self.world.agent_positions().clone()
    }

    #[getter]
    fn wall_pos(&self) -> Vec<Position> {
        self.world.walls().copied().collect()
    }

    #[getter]
    fn gems(&self) -> Vec<(Position, PyGem)> {
        self.world
            .gems()
            .map(|(pos, gem)| (*pos, PyGem::new(gem.clone())))
            .collect()
    }

    #[getter]
    fn lasers(&self) -> Vec<(Position, PyLaser)> {
        self.world
            .lasers()
            .map(|(pos, laser)| (*pos, PyLaser::new(laser.clone())))
            .collect()
    }

    #[getter]
    fn laser_sources(&self) -> Vec<(Position, PyLaserSource)> {
        self.world
            .laser_sources()
            .map(|(pos, laser_source)| (*pos, PyLaserSource::new(laser_source.clone())))
            .collect()
    }

    #[getter]
    fn exit_pos(&self) -> Vec<Position> {
        self.world.exits().copied().collect()
    }

    pub fn step(&mut self, actions: Vec<PyAction>) -> PyResult<i32> {
        let actions: Vec<_> = actions.into_iter().map(|a| a.action).collect();
        match self.world.step(&actions) {
            Ok(r) => Ok(r),
            Err(e) => match e {
                RuntimeWorldError::InvalidAction {
                    agent_id,
                    available,
                    taken,
                } => Err(exceptions::PyValueError::new_err(format!(
                    "Invalid action for agent {agent_id}: available actions: {available:?}, taken action: {taken}",
                ))),
                RuntimeWorldError::WorldIsDone => Err(exceptions::PyValueError::new_err("World is done, cannot step anymore")),
                RuntimeWorldError::InvalidNumberOfActions { given, expected } => Err(exceptions::PyValueError::new_err(format!(
                    "Invalid number of actions: given {given}, expected {expected}",
                ))),
                other => panic!("Unexpected error: {:?}", other),
            },
        }
    }

    pub fn reset(&mut self) {
        self.world.reset()
    }

    pub fn available_actions(&self) -> Vec<Vec<PyAction>> {
        self.world
            .available_actions()
            .iter()
            .map(|a| a.iter().map(|a| PyAction { action: a.clone() }).collect())
            .collect()
    }

    pub fn agents(&self) -> Vec<PyAgent> {
        self.world
            .agents()
            .iter()
            .map(|a| PyAgent { agent: a.clone() })
            .collect()
    }

    #[getter]
    pub fn done(&self) -> bool {
        self.world.done()
    }

    pub fn __deepcopy__(&self, _memo: &PyDict) -> Self {
        self.clone()
    }

    fn get_image(self_: PyRef<'_, Self>) -> PyResult<PyObject> {
        let dims = self_.image_dimensions();
        let dims = (dims.0 as usize, dims.1 as usize, 3);
        let py = self_.py();
        let img = self_.renderer.update(&self_.world);
        let buffer = img.into_raw();
        let res = PyArray1::from_vec(py, buffer).reshape(dims).unwrap();
        Ok(res.into_py(py))
    }
}
