use numpy::PyArray1;
use pyo3::{exceptions, prelude::*, types::PyDict};

use crate::{
    parsing::parse, parsing::ParseError, Position, Renderer, RuntimeWorldError, Tile, World,
};

use super::{
    pyaction::PyAction,
    pyagent::PyAgent,
    pydirection::PyDirection,
    pytile::{PyGem, PyLaser, PyLaserSource},
    pyworld_state::PyWorldState,
};

#[pyclass(unsendable, name = "World")]
#[derive(Clone)]
pub struct PyWorld {
    world: World,
    renderer: Renderer,
}

#[pymethods]
impl PyWorld {
    #[new]
    pub fn new(map_str: String) -> PyResult<Self> {
        let world = match parse(&map_str) {
            Ok(world) => world,
            Err(e) => return Err(parse_error_to_exception(e)),
        };
        let renderer = Renderer::new(&world);
        Ok(PyWorld { world, renderer })
    }

    #[staticmethod]
    fn from_file(filename: String) -> PyResult<Self> {
        let world = match World::from_file(&filename) {
            Ok(world) => world,
            Err(e) => return Err(parse_error_to_exception(e)),
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
    /// The list of the positions of the void tiles
    pub fn void_pos(&self) -> Vec<Position> {
        self.world.void_positions().copied().collect()
    }

    #[getter]
    /// The proportion of agents that have reached an exit tile.
    fn exit_rate(&self) -> f32 {
        let n_arrived = self.world.n_agents_arrived() as f32;
        n_arrived / (self.world.n_agents() as f32)
    }

    #[getter]
    /// The number of gems collected so far (since the last reset).
    fn gems_collected(&self) -> usize {
        self.world.n_gems_collected()
    }

    #[getter]
    /// The positions of the agents.
    fn agents_positions(&self) -> Vec<Position> {
        self.world.agents_positions().clone()
    }

    #[getter]
    /// The positions of the wall tiles.
    fn wall_pos(&self) -> Vec<Position> {
        self.world.walls().copied().collect()
    }

    #[getter]
    /// The gems with their respective position.
    fn gems(&self) -> Vec<(Position, PyGem)> {
        self.world
            .gems()
            .map(|(pos, gem)| (*pos, PyGem::new(gem.agent(), gem.is_collected())))
            .collect()
    }

    #[getter]
    /// The lasers with their respective position.
    fn lasers(&self) -> Vec<(Position, PyLaser)> {
        self.world
            .lasers()
            .map(|(pos, laser)| {
                (
                    *pos,
                    PyLaser::new(
                        laser.is_on(),
                        PyDirection::new(laser.direction()),
                        laser.agent_id(),
                        laser.agent(),
                    ),
                )
            })
            .collect()
    }

    #[getter]
    /// The laser sources with their respective position.
    fn laser_sources(&self) -> Vec<(Position, PyLaserSource)> {
        self.world
            .laser_sources()
            .map(|(pos, laser_source)| (*pos, PyLaserSource::new(laser_source.clone())))
            .collect()
    }

    #[getter]
    /// The positions of the exit tiles.
    fn exit_pos(&self) -> Vec<Position> {
        self.world.exits().map(|(pos, _)| pos).copied().collect()
    }

    /// Perform a step in the world and returns the reward for that transition.
    pub fn step(&mut self, actions: Vec<PyAction>) -> PyResult<f32> {
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

    /// Reset the world to its original state.
    pub fn reset(&mut self) {
        self.world.reset();
    }

    /// Return the available actions for each agent.
    /// `world.available_actions()[i]` is the list of available actions for agent i.
    pub fn available_actions(&self) -> Vec<Vec<PyAction>> {
        self.world
            .available_actions()
            .iter()
            .map(|a| a.iter().map(|a| PyAction { action: a.clone() }).collect())
            .collect()
    }

    #[getter]
    /// Return the list of agents.
    pub fn agents(&self) -> Vec<PyAgent> {
        self.world
            .agents()
            .iter()
            .map(|a| PyAgent { agent: a.clone() })
            .collect()
    }

    #[getter]
    /// Whether the last transition yielded a terminal state.
    pub fn done(&self) -> bool {
        self.world.done()
    }

    pub fn __deepcopy__(&self, _memo: &PyDict) -> Self {
        self.clone()
    }

    /// Renders the world as an image and returns it in a numpy array.
    fn get_image(self_: PyRef<'_, Self>) -> PyResult<PyObject> {
        let dims = self_.image_dimensions();
        let dims = (dims.1 as usize, dims.0 as usize, 3);
        let py = self_.py();
        let img = self_.renderer.update(&self_.world);
        let buffer = img.into_raw();
        let res = PyArray1::from_vec(py, buffer)
            .reshape(dims)
            .unwrap_or_else(|_| panic!("Could not reshape the image to {dims:?}"));
        Ok(res.into_py(py))
    }

    /// Force the world to a specific state
    fn set_state(&mut self, state: PyWorldState) -> PyResult<()> {
        match self.world.force_state(&state.into()) {
            Ok(_) => Ok(()),
            Err(e) => Err(exceptions::PyValueError::new_err(format!("{e:?}"))),
        }
    }

    /// Return the current state of the world (that can be set with `world.set_state`)
    fn get_state(&self) -> PyWorldState {
        let state = self.world.get_state();
        PyWorldState::new(state.agents_positions, state.gems_collected)
    }
}

fn parse_error_to_exception(error: ParseError) -> PyErr {
    match error {
        ParseError::InvalidFileName { file_name } => {
            exceptions::PyFileNotFoundError::new_err(file_name)
        }

        ParseError::DuplicateStartTile {
            agent_id,
            start1,
            start2,
        } => exceptions::PyValueError::new_err(format!(
            "Agent {agent_id} has two start tiles: {start1:?} and {start2:?}"
        )),
        ParseError::InconsistentDimensions {
            expected_n_cols,
            actual_n_cols,
            row,
        } => exceptions::PyValueError::new_err(format!(
            "Inconsistent number of columns in row {}: expected {}, got {}",
            row, expected_n_cols, actual_n_cols
        )),
        ParseError::NotEnoughExitTiles { n_starts, n_exits } => exceptions::PyValueError::new_err(
            format!("Not enough exit tiles: {n_starts} starts, {n_exits} exits"),
        ),
        ParseError::EmptyWorld => exceptions::PyValueError::new_err("Empty world: no tiles"),
        ParseError::NoAgents => exceptions::PyValueError::new_err("No agents in the world"),

        ParseError::InvalidTile {
            tile_str,
            line,
            col,
        } => exceptions::PyValueError::new_err(format!(
            "Invalid tile '{tile_str}' at position ({line}, {col})"
        )),
    }
}
