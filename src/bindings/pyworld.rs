use numpy::PyArray1;
use pyo3::{exceptions, prelude::*, types::PyDict};

use crate::{
    parsing::parse, parsing::ParseError, Position, Renderer, RuntimeWorldError, Tile, World,
    WorldState,
};

use super::{
    pyaction::PyAction,
    pyagent::PyAgent,
    pydirection::PyDirection,
    pytile::{PyGem, PyLaser, PyLaserSource},
    pyworld_state::PyWorldState,
};

#[pyclass(name = "World", module = "lle")]
#[derive(Clone)]
pub struct PyWorld {
    world: World,
    renderer: Renderer,
}

unsafe impl Send for PyWorld {}

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

    #[staticmethod]
    fn level(level: usize) -> PyResult<Self> {
        match World::get_level(level) {
            Ok(world) => {
                let renderer = Renderer::new(&world);
                Ok(PyWorld { world, renderer })
            }
            Err(err) => Err(parse_error_to_exception(err)),
        }
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

    /// Renders the world as an image and returns it in a numpy array.
    fn get_image(&self, py: Python) -> PyResult<PyObject> {
        let dims = self.image_dimensions();
        let dims = (dims.1 as usize, dims.0 as usize, 3);
        let img = self.renderer.update(&self.world);
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

    pub fn __deepcopy__(&self, _memo: &PyDict) -> Self {
        self.clone()
    }

    /// This method is called to instantiate the object before deserialisation.
    /// It required "default arguments" to be provided to the __new__ method
    /// before replacing them by the actual values in __setstate__.
    pub fn __getnewargs__(&self) -> PyResult<(String,)> {
        Ok((String::from("S0 X"),))
    }

    /// Enable serialisation with pickle
    pub fn __getstate__(&self) -> PyResult<(String, Vec<bool>, Vec<Position>)> {
        let state = self.world.get_state();
        let data = (
            self.world_string(),
            state.gems_collected.clone(),
            state.agents_positions.clone(),
        );
        Ok(data)
    }

    pub fn __setstate__(&mut self, state: (String, Vec<bool>, Vec<Position>)) -> PyResult<()> {
        let world = match parse(&state.0) {
            Ok(world) => world,
            Err(e) => panic!("Could not parse the world: {:?}", e),
        };
        self.world = world;
        self.renderer = Renderer::new(&self.world);
        self.world
            .force_state(&WorldState {
                gems_collected: state.1,
                agents_positions: state.2,
            })
            .unwrap();
        Ok(())
    }

    // /// Enable pickle serialisation
    // pub fn __getstate__(self_: PyRef<'_, Self>) -> &'_ PyDict {
    //     let py = self_.py();
    //     let res = PyDict::new(py);
    //     res.set_item("world_str", self_.world_string()).unwrap();
    //     let state = self_.world.get_state();
    //     res.set_item("gems_collected", state.gems_collected)
    //         .unwrap();
    //     res.set_item("agents_positions", state.agents_positions)
    //         .unwrap();
    //     println!("getstate: {:?}", res);
    //     res
    // }
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
        ParseError::InvalidLevel { asked, min, max } => exceptions::PyValueError::new_err(format!(
            "Invalid level: {asked}. Expected a level between {min} and {max}.",
            asked = asked,
            min = min,
            max = max
        )),
    }
}
