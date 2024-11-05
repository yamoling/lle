use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use itertools::izip;
use numpy::{PyArray1, PyArrayMethods};
use pyo3::{
    exceptions::PyTypeError,
    prelude::*,
    types::{PyDict, PyTuple},
};
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pymethods};

use crate::{Action, Renderer, World};

use super::{
    pyaction::PyAction,
    pyagent::PyAgent,
    pyevent::PyWorldEvent,
    pyexceptions::{parse_error_to_exception, runtime_error_to_pyexception},
    pyposition::PyPosition,
    pytile::{PyGem, PyLaser, PyLaserSource},
    pyworld_state::PyWorldState,
};

// Implementation notes:
// - The `PyWorld` struct is a wrapper around the `World` struct.
// - To make it thread-safe, we wrap the `World` struct in an `Arc<Mutex<World>>`.
// - Everything that is immutable is directly accessible from the `World` struct.
// - Everything that is mutable is accessed through the `Arc<Mutex<World>>`.

/// The `World` represents the environment in which the agents evolve.
/// A world is created from a string where each character represents a tile.
/// There are 6 predefined levels for convenience.
///
/// ```python
/// from lle import World
/// # Create from a predefined level
/// w1 = World.level(5)
/// # Create from a file
/// w2 = World.from_file("my_map.txt")
/// # Create from a string
/// w3 = World("S0 X")
/// ```
#[gen_stub_pyclass]
#[pyclass(name = "World", module = "lle", subclass)]
pub struct PyWorld {
    /// The positions of the exits tiles.
    #[pyo3(get)]
    exit_pos: Vec<PyPosition>,
    /// The possible random start positions of each agent.
    #[pyo3(get)]
    random_start_pos: Vec<Vec<PyPosition>>,
    /// The positions of the walls.
    #[pyo3(get)]
    wall_pos: Vec<PyPosition>,
    /// The positions of the void tiles.
    #[pyo3(get)]
    void_pos: Vec<PyPosition>,
    /// The height of the world (in number of tiles).
    #[pyo3(get)]
    height: usize,
    /// The width of the world (in number of tiles).
    #[pyo3(get)]
    width: usize,
    /// The number of gems in the world.
    #[pyo3(get)]
    n_gems: usize,
    /// The number of agents in the world.
    #[pyo3(get)]
    n_agents: usize,
    world: Arc<Mutex<World>>,
    renderer: Renderer,
}

/// The `PyWorld` struct is thread-safe because:
///  - the `World` struct is wrapped in an `Arc<Mutex<_>>`
///  - the other fields are immutable
unsafe impl Send for PyWorld {}

impl From<World> for PyWorld {
    fn from(world: World) -> Self {
        let renderer = Renderer::new(&world);
        PyWorld {
            exit_pos: world
                .exits_positions()
                .into_iter()
                .map(|p| p.into())
                .collect(),
            random_start_pos: world
                .possible_starts()
                .into_iter()
                .map(|p| p.into_iter().map(|p| p.as_ij()).collect())
                .collect(),
            wall_pos: world.walls().into_iter().map(|p| p.into()).collect(),
            void_pos: world
                .void_positions()
                .into_iter()
                .map(|p| p.into())
                .collect(),
            height: world.height(),
            width: world.width(),
            n_gems: world.n_gems(),
            n_agents: world.n_agents(),
            renderer,
            world: Arc::new(Mutex::new(world)),
        }
    }
}

#[gen_stub_pymethods]
#[pymethods]
impl PyWorld {
    #[new]
    pub fn new(map_str: String) -> PyResult<Self> {
        match World::try_from(map_str) {
            Ok(world) => Ok(PyWorld::from(world)),
            Err(e) => Err(parse_error_to_exception(e)),
        }
    }

    /// Constructs a World from a string.
    ///
    /// Raises:
    ///     - `RuntimeError`: if the file is not a valid level.
    ///     - `ValueError` if the file is not a valid level (inconsistent dimensions or invalid grid).
    #[allow(unused_variables)]
    pub fn __init__(&self, map_str: String) {
        // Just to have the __init__ method in the generated stub
    }

    /// Parse the content of `filename` to create a World.
    ///
    /// The file can either be a toml or a plain text file.
    /// Raises:
    ///     - `FileNotFoundError`: if the file does not exist.
    #[staticmethod]
    fn from_file(filename: String) -> PyResult<Self> {
        let world = match World::from_file(&filename) {
            Ok(world) => world,
            Err(e) => return Err(parse_error_to_exception(e)),
        };
        Ok(PyWorld::from(world))
    }

    /// Retrieve the standard level (between `1` and `6`).
    /// Raises:
    ///     - `ValueError`: if the level is invalid.
    #[staticmethod]
    fn level(level: usize) -> PyResult<Self> {
        match World::get_level(level) {
            Ok(world) => Ok(PyWorld::from(world)),
            Err(err) => Err(parse_error_to_exception(err)),
        }
    }

    /// The string upon which the world has been constructed.
    #[getter]
    fn world_string(&self) -> String {
        self.world.lock().unwrap().initial_world_string().into()
    }

    /// The dimensions (in pixels) of the image redered (width, height)
    #[getter]
    pub fn image_dimensions(&self) -> (u32, u32) {
        (self.renderer.pixel_width(), self.renderer.pixel_height())
    }

    /// The number of gems collected by the agents so far since the last reset.
    #[getter]
    fn gems_collected(&self) -> usize {
        self.world.lock().unwrap().n_gems_collected()
    }

    /// The (i, j) position of each agent.
    #[getter]
    fn agents_positions(&self) -> Vec<PyPosition> {
        self.world
            .lock()
            .unwrap()
            .agents_positions()
            .into_iter()
            .map(|p| (*p).into())
            .collect()
    }

    /// The gems with their respective position.
    #[getter]
    fn gems(&self) -> HashMap<PyPosition, PyGem> {
        let arc_world = self.world.clone();
        let world = self.world.lock().unwrap();
        izip!(world.gems_positions(), world.gems())
            .into_iter()
            .map(|(pos, gem)| (pos.into(), PyGem::new(gem, pos.into(), arc_world.clone())))
            .collect()
    }

    /// The (i, j) position of every laser.
    /// Since two lasers can cross, there can be duplicates in the positions.
    #[getter]
    fn lasers(&self) -> Vec<(PyPosition, PyLaser)> {
        let arc_world = self.world.clone();
        let world = self.world.lock().unwrap();
        world
            .lasers()
            .iter()
            .map(|(pos, laser)| ((*pos).into(), PyLaser::new(laser, *pos, arc_world.clone())))
            .collect()
    }

    /// A mapping from (i, j) positions to laser sources.
    #[getter]
    fn laser_sources(&self) -> HashMap<PyPosition, PyLaserSource> {
        let arc_world = self.world.clone();
        let world = self.world.lock().unwrap();
        world
            .sources()
            .iter()
            .map(|(pos, laser_source)| {
                (
                    (*pos).into(),
                    PyLaserSource::new(arc_world.clone(), pos.into(), laser_source),
                )
            })
            .collect()
    }

    #[getter]
    /// The start position of each agent for this reset.
    fn start_pos(&self) -> Vec<PyPosition> {
        let world = self.world.lock().unwrap();
        world.starts().iter().map(|p| (*p).into()).collect()
    }

    /// Simultaneously perform an action for each agent in the world.
    /// Performing a step generates events (see `WorldEvent`) to give information about the consequences of the joint action.
    ///
    /// Args:
    ///    action: The action to perform for each agent. A single action is also accepted if there is a single agent in the world.
    ///
    /// Returns:
    ///   The list of events that occurred while agents took their action.
    ///
    /// Raises:
    ///     - `InvalidActionError` if an agent takes an action that is not available.
    ///     - `ValueError` if the number of actions is different from the number of agents
    ///
    /// Example:
    /// ```python
    /// world = World("S1 G X S0 X")
    /// world.reset()
    /// events = world.step([Action.STAY, Action.EAST])
    /// assert len(events) == 1
    /// assert events[0].agent_id == 1
    /// assert events[0].event_type == EventType.GEM_COLLECTED
    ///
    /// events = world.step([Action.EAST, Action.EAST])
    /// assert len(events) == 2
    /// assert all(e.event_type == EventType.AGENT_EXIT for e in events)
    /// ```
    pub fn step(&mut self, py: Python, action: PyObject) -> PyResult<Vec<PyWorldEvent>> {
        // Check if action is a list or a single action
        let actions: Vec<PyAction> = if let Ok(actions) = action.extract::<Vec<PyAction>>(py) {
            actions
        } else if let Ok(action) = action.extract::<PyAction>(py) {
            vec![action]
        } else {
            return Err(PyTypeError::new_err(
                "Action must be of type Action or list[Action]",
            ));
        };

        let actions: Vec<Action> = actions.into_iter().map(|a| a.into()).collect();
        match self.world.lock().unwrap().step(&actions) {
            Ok(events) => {
                let events: Vec<PyWorldEvent> =
                    events.iter().map(|e| PyWorldEvent::from(e)).collect();
                Ok(events)
            }
            Err(e) => Err(runtime_error_to_pyexception(e)),
        }
    }
    /// Reset the world to its original state.
    /// This should be done directly after creating the world.
    pub fn reset(&mut self) {
        self.world.lock().unwrap().reset();
    }

    /// Compute the list of available actions at the current time step for each agent.
    /// The actions available for agent `n` are given by `world.available_actions()[n]`.
    /// Returns:
    ///    The list of available actions for each agent.
    pub fn available_actions(&self) -> Vec<Vec<PyAction>> {
        self.world
            .lock()
            .unwrap()
            .available_actions()
            .iter()
            .map(|a| a.iter().map(|a| PyAction::from(a)).collect())
            .collect()
    }

    /// Compute the list of available joint actions at the current time step.
    /// The result has shape (x, n_agents) where x is the number of joint actions available.
    /// Returns:
    ///   The list of available joint actions.
    ///
    /// Example:
    /// ```python
    /// world = World(". .  .  . .\n. S0 . S1 .\n. X  .  X .\n")
    /// world.reset()
    /// assert len(world.available_joint_actions()) == len(Action.ALL) ** 2
    /// ```
    pub fn available_joint_actions(&self) -> Vec<Vec<PyAction>> {
        self.world
            .lock()
            .unwrap()
            .available_joint_actions()
            .iter()
            .map(|a| a.iter().map(|a| PyAction::from(a)).collect())
            .collect()
    }

    /// The list of agents in the world.
    #[getter]
    pub fn agents(&self) -> Vec<PyAgent> {
        self.world
            .lock()
            .unwrap()
            .agents()
            .iter()
            .map(|a| PyAgent { agent: a.clone() })
            .collect()
    }

    /// Renders the world as an image and returns it in a numpy array.
    /// Returns:
    ///     The image of the world as a numpy array of shape (height * 32, width * 32, 3) with type uint8.
    fn get_image<'a>(
        &'a self,
        py: Python<'a>,
    ) -> Bound<'a, numpy::PyArray<u8, numpy::ndarray::Dim<[usize; 3]>>> {
        let dims = self.image_dimensions();
        let dims = (dims.1 as usize, dims.0 as usize, 3);
        let img = self.renderer.update(&self.world.lock().unwrap());
        let buffer = img.into_raw();
        PyArray1::from_vec_bound(py, buffer).reshape(dims).unwrap()
    }

    /// Force the world to a given state
    /// Args:
    ///     state: The state to set the world to.
    /// Returns:
    ///     The list of events that occurred while agents entered their state.
    /// Raises:
    ///     - `InvalidWorldStateError`: if the state is invalid.
    fn set_state(&mut self, state: PyWorldState) -> PyResult<Vec<PyWorldEvent>> {
        match self.world.lock().unwrap().set_state(&state.into()) {
            Ok(events) => Ok(events.iter().map(|e| PyWorldEvent::from(e)).collect()),
            Err(e) => Err(runtime_error_to_pyexception(e)),
        }
    }

    /// Return the current state of the world.
    fn get_state(&self) -> PyWorldState {
        let state = self.world.lock().unwrap().get_state();
        state.into()
    }

    /// Returns a deep copy of the object.
    ///
    /// Example:
    /// ```python
    /// from copy import deepcopy
    /// world = World("S0 X")
    /// world.reset()
    /// world_copy = deepcopy(world)
    /// world.step(Action.EAST)
    /// assert world.get_state() != world_copy.get_state()
    /// ```
    pub fn __deepcopy__(&self, _memo: &Bound<PyDict>) -> Self {
        self.clone()
    }

    /// This method is called to instantiate the object before deserialisation.
    /// It required "default arguments" to be provided to the __new__ method
    /// before replacing them by the actual values in __setstate__.
    pub fn __getnewargs__(&self, py: Python) -> PyObject {
        PyTuple::new_bound(py, vec![String::from("S0 X")].iter()).into()
    }

    /// Enable serialisation with pickle
    pub fn __getstate__(&self) -> PyResult<(String, PyWorldState)> {
        let world = self.world.lock().unwrap();
        let state: PyWorldState = world.get_state().into();
        let world_string = world.compute_world_string().to_owned();
        Ok((world_string, state))
    }

    /// Enable deserialisation with pickle
    pub fn __setstate__(&mut self, state: (String, PyWorldState)) -> PyResult<()> {
        let world = match World::try_from(state.0) {
            Ok(mut w) => {
                w.set_state(&state.1.into()).unwrap();
                w
            }
            Err(e) => panic!("Could not parse the world: {:?}", e),
        };
        self.renderer = Renderer::new(&world);
        self.n_agents = world.n_agents();
        self.n_gems = world.n_gems();
        self.height = world.height();
        self.width = world.width();
        self.exit_pos = world
            .exits_positions()
            .iter()
            .map(|p| (*p).into())
            .collect();
        self.random_start_pos = world
            .possible_starts()
            .iter()
            .map(|p| p.iter().map(|p| p.as_ij()).collect())
            .collect();
        self.wall_pos = world.walls().iter().map(|p| (*p).into()).collect();
        self.void_pos = world.void_positions().iter().map(|p| (*p).into()).collect();
        self.world = Arc::new(Mutex::new(world));
        Ok(())
    }

    pub fn __repr__(&self) -> String {
        let mut res = format!(
            "World(height={}, width={}, n_gems={}, n_agents={})",
            self.height, self.width, self.n_gems, self.n_agents
        );
        let w = self.world.lock().unwrap();
        res.push_str(
            &w.agents_positions()
                .iter()
                .enumerate()
                .fold(String::new(), |acc, (i, pos)| {
                    format!("{}Agent {} position: {:?}, ", acc, i, pos)
                }),
        );
        res
    }
}

impl Clone for PyWorld {
    fn clone(&self) -> Self {
        let core = self.world.lock().unwrap().clone();
        let renderer = Renderer::new(&core);
        PyWorld {
            exit_pos: self.exit_pos.clone(),
            random_start_pos: self.random_start_pos.clone(),
            wall_pos: self.wall_pos.clone(),
            void_pos: self.void_pos.clone(),
            height: self.height,
            width: self.width,
            n_gems: self.n_gems,
            n_agents: self.n_agents,
            world: Arc::new(Mutex::new(core)),
            renderer,
        }
    }
}
