use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use itertools::izip;
use numpy::{PyArray1, PyArrayMethods};
use pyo3::{exceptions::PyTypeError, prelude::*, types::PyDict};

use crate::{Position, Renderer, World, WorldState};

use super::{
    pyaction::PyAction,
    pyagent::PyAgent,
    pyevent::PyWorldEvent,
    pyexceptions::{parse_error_to_exception, runtime_error_to_pyexception},
    pytile::{PyGem, PyLaser, PyLaserSource},
    pyworld_state::PyWorldState,
};

// Implementation notes:
// - The `PyWorld` struct is a wrapper around the `World` struct.
// - To make it thread-safe, we wrap the `World` struct in an `Arc<Mutex<World>>`.
// - Everything that is immutable is directly accessible from the `World` struct.
// - Everything that is mutable is accessed through the `Arc<Mutex<World>>`.

#[pyclass(name = "World", module = "lle", subclass)]
pub struct PyWorld {
    #[pyo3(get)]
    exit_pos: Vec<Position>,
    #[pyo3(get)]
    start_pos: Vec<Position>,
    #[pyo3(get)]
    wall_pos: Vec<Position>,
    #[pyo3(get)]
    void_pos: Vec<Position>,
    #[pyo3(get)]
    height: usize,
    #[pyo3(get)]
    width: usize,
    #[pyo3(get)]
    n_gems: usize,
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
            exit_pos: world.exits_positions(),
            start_pos: world.starts(),
            wall_pos: world.walls(),
            void_pos: world.void_positions(),
            height: world.height(),
            width: world.width(),
            n_gems: world.n_gems(),
            n_agents: world.n_agents(),
            renderer,
            world: Arc::new(Mutex::new(world)),
        }
    }
}

#[pymethods]
impl PyWorld {
    #[new]
    pub fn new(map_str: String) -> PyResult<Self> {
        match World::try_from(map_str) {
            Ok(world) => Ok(PyWorld::from(world)),
            Err(e) => Err(parse_error_to_exception(e)),
        }
    }

    #[staticmethod]
    fn from_file(filename: String) -> PyResult<Self> {
        let world = match World::from_file(&filename) {
            Ok(world) => world,
            Err(e) => return Err(parse_error_to_exception(e)),
        };
        Ok(PyWorld::from(world))
    }

    #[staticmethod]
    fn level(level: usize) -> PyResult<Self> {
        match World::get_level(level) {
            Ok(world) => Ok(PyWorld::from(world)),
            Err(err) => Err(parse_error_to_exception(err)),
        }
    }

    #[getter]
    fn world_string(&self) -> String {
        self.world.lock().unwrap().initial_world_string().into()
    }

    #[getter]
    /// Return the rendering dimensions (width, height)
    pub fn image_dimensions(&self) -> (u32, u32) {
        (self.renderer.pixel_width(), self.renderer.pixel_height())
    }

    #[getter]
    /// The number of gems collected so far (since the last reset).
    fn gems_collected(&self) -> usize {
        self.world.lock().unwrap().n_gems_collected()
    }

    #[getter]
    /// The positions of the agents.
    fn agents_positions(&self) -> Vec<Position> {
        self.world.lock().unwrap().agents_positions().clone()
    }

    #[getter]
    /// The gems with their respective position.
    fn gems(&self) -> HashMap<Position, PyGem> {
        let arc_world = self.world.clone();
        let world = self.world.lock().unwrap();
        izip!(world.gems_positions(), world.gems())
            .into_iter()
            .map(|(pos, gem)| (pos, PyGem::new(gem, pos, arc_world.clone())))
            .collect()
    }

    #[getter]
    /// The lasers with their respective position.
    fn lasers(&self) -> Vec<(Position, PyLaser)> {
        let arc_world = self.world.clone();
        let world = self.world.lock().unwrap();
        world
            .lasers()
            .iter()
            .map(|(pos, laser)| (*pos, PyLaser::new(laser, *pos, arc_world.clone())))
            .collect()
    }

    #[getter]
    /// The laser sources with their respective position.
    fn laser_sources(&self) -> HashMap<Position, PyLaserSource> {
        let arc_world = self.world.clone();
        let world = self.world.lock().unwrap();
        world
            .sources()
            .iter()
            .map(|(pos, laser_source)| {
                (
                    *pos,
                    PyLaserSource::new(arc_world.clone(), *pos, laser_source),
                )
            })
            .collect()
    }

    /// Perform a step in the world and returns the events that happened during that transition.
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

        let actions: Vec<_> = actions.into_iter().map(|a| a.action).collect();
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
    pub fn reset(&mut self) {
        self.world.lock().unwrap().reset();
    }

    /// Return the available actions for each agent.
    /// `world.available_actions()[i]` is the list of available actions for agent i.
    pub fn available_actions(&self) -> Vec<Vec<PyAction>> {
        self.world
            .lock()
            .unwrap()
            .available_actions()
            .iter()
            .map(|a| a.iter().map(|a| PyAction { action: a.clone() }).collect())
            .collect()
    }

    /// Return the available joint actions with shape (x, n_agents) where x is the number of joint actions.
    pub fn available_joint_actions(&self) -> Vec<Vec<PyAction>> {
        self.world
            .lock()
            .unwrap()
            .available_joint_actions()
            .iter()
            .map(|a| a.iter().map(|a| PyAction { action: a.clone() }).collect())
            .collect()
    }

    #[getter]
    /// Return the list of agents.
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
    fn get_image(&self, py: Python) -> PyResult<PyObject> {
        let dims = self.image_dimensions();
        let dims = (dims.1 as usize, dims.0 as usize, 3);
        let img = self.renderer.update(&self.world.lock().unwrap());
        let buffer = img.into_raw();
        let res = PyArray1::from_vec_bound(py, buffer)
            .reshape(dims)
            .unwrap_or_else(|_| panic!("Could not reshape the image to {dims:?}"));
        Ok(res.into_py(py))
    }

    /// Force the world to a specific state
    fn set_state(&mut self, state: PyWorldState) -> PyResult<Vec<PyWorldEvent>> {
        match self.world.lock().unwrap().set_state(&state.into()) {
            Ok(events) => Ok(events.iter().map(|e| PyWorldEvent::from(e)).collect()),
            Err(e) => Err(runtime_error_to_pyexception(e)),
        }
    }

    /// Return the current state of the world (that can be set with `world.set_state`)
    fn get_state(&self) -> PyWorldState {
        let state = self.world.lock().unwrap().get_state();
        state.into()
    }

    pub fn __deepcopy__(&self, _memo: &Bound<PyDict>) -> Self {
        self.clone()
    }

    /// This method is called to instantiate the object before deserialisation.
    /// It required "default arguments" to be provided to the __new__ method
    /// before replacing them by the actual values in __setstate__.
    pub fn __getnewargs__(&self) -> PyResult<(String,)> {
        Ok((String::from("S0 X"),))
    }

    /// Enable serialisation with pickle
    pub fn __getstate__(&self) -> PyResult<(String, Vec<bool>, Vec<Position>, Vec<bool>)> {
        let world = self.world.lock().unwrap();
        let state = world.get_state();
        let data = (
            world.compute_world_string().to_owned(),
            state.gems_collected.clone(),
            state.agents_positions.clone(),
            state.agents_alive.clone(),
        );
        Ok(data)
    }

    pub fn __setstate__(
        &mut self,
        state: (String, Vec<bool>, Vec<Position>, Vec<bool>),
    ) -> PyResult<()> {
        let mut world = match World::try_from(state.0) {
            Ok(w) => w,
            Err(e) => panic!("Could not parse the world: {:?}", e),
        };
        self.renderer = Renderer::new(&world);
        world
            .set_state(&WorldState {
                gems_collected: state.1,
                agents_positions: state.2,
                agents_alive: state.3,
            })
            .unwrap();
        self.world = Arc::new(Mutex::new(world));
        Ok(())
    }

    pub fn __repr__(&self) -> String {
        let mut res = format!(
            "World(height={}, width={}, n_gems={}, n_agents={}, ",
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
            start_pos: self.start_pos.clone(),
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
