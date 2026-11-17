use numpy::{PyArray1, PyArray2, PyArrayMethods};
use pyo3::{
    IntoPyObjectExt,
    exceptions::{PyIndexError, PyValueError},
    prelude::*,
    types::{PyDict, PyTuple},
};
use pyo3_stub_gen::{
    PyStubType, TypeInfo,
    derive::{gen_stub_pyclass, gen_stub_pymethods},
};

use super::{PyContinuousWorldState, PyPhysicsParams};
use crate::bindings::{
    pyexceptions::{InvalidLevelError, continuous_error_to_pyexception, parse_error_to_exception},
    tiles::PyDirection,
    world::{PyPosition, PyWorldEvent},
};
use crate::continuous::{ContinuousAction, ContinuousWorld, RNG_STATE_SIZE};
use crate::shared::{MapSpec, ParseError, Position, map::levels::LEVELS};

/// What `ContinuousWorld.__getstate__` returns: the map string, the parameters, the state and the
/// state of the random number generator.
type PickledWorld = (String, PyPhysicsParams, PyContinuousWorldState, Vec<u8>);

/// The outcome of `ContinuousWorld.step`: the events of every tick, in order.
#[gen_stub_pyclass]
#[pyclass(
    name = "StepOutcome",
    module = "lle.continuous.world",
    skip_from_py_object
)]
pub struct PyStepOutcome {
    /// The events, in the order in which they happened.
    #[pyo3(get)]
    events: Vec<PyWorldEvent>,
    /// The zero-based index of the tick of each event, within the step.
    #[pyo3(get)]
    ticks: Vec<usize>,
    /// The number of ticks actually run: fewer than asked when the episode ended early (an agent
    /// died, or every alive agent has arrived).
    #[pyo3(get)]
    n_ticks: usize,
}

#[gen_stub_pymethods]
#[pymethods]
impl PyStepOutcome {
    /// The number of ticks and every event with its tick index.
    ///
    /// @ai-generated
    fn __repr__(&self) -> String {
        let events: Vec<String> = self
            .events
            .iter()
            .zip(&self.ticks)
            .map(|(e, t)| format!("({t}, {})", e.__str__()))
            .collect();
        format!(
            "StepOutcome(n_ticks={}, events=[{}])",
            self.n_ticks,
            events.join(", ")
        )
    }
}

/// The continuous counterpart of `World`: agents are discs that move freely in the plane of the
/// grid, at the scale of one unit per cell.
///
/// Positions are `(x, y)` pairs: `x` grows eastwards (columns) and `y` southwards (rows), so
/// that the centre of the cell `(i, j)` is `(j + 0.5, i + 0.5)`. An action is an `(ax, ay)` pair
/// in `[-1, 1]²` in the same order (`ax` east, `ay` south), and a joint action has the shape
/// `(n_agents, 2)`.
///
/// Maps are written as for `World` (v1 strings or TOML), but boxes, lifts, buttons, levers,
/// pressure plates and several layers are not supported yet.
///
/// ```python
/// from lle.continuous import ContinuousWorld
/// world = ContinuousWorld("S0 . G X")
/// world.reset()
/// outcome = world.step([[1.0, 0.0]], n_ticks=10)  # move east for 10 ticks
/// print(world.agents_positions)                    # [(1.5, 0.5)]
/// ```
#[gen_stub_pyclass]
#[pyclass(name = "ContinuousWorld", module = "lle.continuous.world")]
pub struct PyContinuousWorld {
    world: ContinuousWorld,
    /// The map, as a string that `MapSpec::parse` reads back (to pickle the world).
    map_str: String,
}

impl PyContinuousWorld {
    /// Build the world of a loaded map with `params` (the default parameters if `None`).
    ///
    /// @ai-generated
    fn build(spec: Result<MapSpec, ParseError>, params: Option<PyPhysicsParams>) -> PyResult<Self> {
        let spec = spec.map_err(parse_error_to_exception)?;
        let map_str = spec.to_string();
        let params = params.map(|p| p.inner).unwrap_or_default();
        let world =
            ContinuousWorld::from_spec(spec, params).map_err(continuous_error_to_pyexception)?;
        Ok(Self { world, map_str })
    }

    /// Convert an array-like of shape `(n_agents, 2)` into actions.
    ///
    /// @ai-generated
    fn extract_actions(actions: &Bound<'_, PyAny>) -> PyResult<Vec<ContinuousAction>> {
        let py = actions.py();
        let kwargs = PyDict::new(py);
        kwargs.set_item("dtype", "float64")?;
        let array = py
            .import("numpy")?
            .call_method("asarray", (actions,), Some(&kwargs))?;
        let shape_error = |shape: &[usize]| {
            PyValueError::new_err(format!(
                "Expected actions of shape (n_agents, 2), got an array of shape {shape:?}"
            ))
        };
        let array = match array.cast_into::<PyArray2<f64>>() {
            Ok(array) => array,
            Err(error) => {
                let shape: Vec<usize> = error.into_inner().getattr("shape")?.extract()?;
                return Err(shape_error(&shape));
            }
        };
        let array = array.readonly();
        let view = array.as_array();
        if view.ncols() != 2 {
            return Err(shape_error(view.shape()));
        }
        Ok(view.rows().into_iter().map(|r| [r[0], r[1]]).collect())
    }
}

/// A Python integer read as a `u64`: `Some(value)` when it fits, `None` for any other integer
/// (negative or too large), so that each method raises its own error instead of
/// `OverflowError`. A value that is not an integer (no `__index__`) raises `TypeError`.
#[derive(Clone, Copy, Debug)]
struct UnsignedInt(Option<u64>);

impl UnsignedInt {
    fn as_usize(self) -> Option<usize> {
        self.0.and_then(|v| usize::try_from(v).ok())
    }
}

impl FromPyObject<'_, '_> for UnsignedInt {
    type Error = PyErr;

    /// `Some` for an integer that fits in a `u64`, `None` for another integer, and `TypeError`
    /// for a value that is not an integer.
    ///
    /// @ai-generated
    fn extract(value: Borrowed<'_, '_, PyAny>) -> Result<Self, Self::Error> {
        if let Ok(value) = value.extract::<u64>() {
            return Ok(Self(Some(value)));
        }
        // An integer out of range, or not an integer at all: `operator.index` tells them apart.
        value
            .py()
            .import("operator")?
            .call_method1("index", (value,))?;
        Ok(Self(None))
    }
}

impl<'py> IntoPyObject<'py> for UnsignedInt {
    type Target = PyAny;
    type Output = Bound<'py, PyAny>;
    type Error = PyErr;

    fn into_pyobject(self, py: Python<'py>) -> Result<Self::Output, Self::Error> {
        match self.0 {
            Some(value) => value.into_bound_py_any(py),
            None => Ok(py.None().into_bound(py)),
        }
    }
}

impl PyStubType for UnsignedInt {
    fn type_output() -> TypeInfo {
        TypeInfo::builtin("int")
    }
}

fn positions(cells: &[Position]) -> Vec<PyPosition> {
    cells.iter().map(|&p| p.into()).collect()
}

#[gen_stub_pymethods]
#[pymethods]
impl PyContinuousWorld {
    /// Create a world from a map string (v1 or TOML) and physics parameters (the default ones if
    /// `None`). The world is reset.
    ///
    /// Raises:
    ///     ParsingError: if the map is invalid.
    ///     UnsupportedFeatureError: if the map uses a feature that the continuous engine does not
    ///         support (boxes, lifts, buttons, levers, pressure plates or several layers).
    ///     ValueError: if the parameters are invalid.
    ///
    /// @ai-generated
    #[new]
    #[pyo3(signature = (map_str, params=None))]
    fn new(map_str: &str, params: Option<PyPhysicsParams>) -> PyResult<Self> {
        Self::build(MapSpec::parse(map_str), params)
    }

    /// The built-in level `n` (between 1 and 6), with the given physics parameters (the default
    /// ones if `None`).
    ///
    /// Raises:
    ///     InvalidLevelError: if the level does not exist.
    ///     UnsupportedFeatureError, ValueError: as the constructor.
    ///
    /// @ai-generated
    #[staticmethod]
    #[pyo3(signature = (n, params=None))]
    fn level(
        #[gen_stub(override_type(type_repr = "builtins.int"))] n: &Bound<'_, PyAny>,
        params: Option<PyPhysicsParams>,
    ) -> PyResult<Self> {
        match n.extract::<UnsignedInt>()?.as_usize() {
            Some(n) => Self::build(MapSpec::level(n), params),
            None => Err(InvalidLevelError::new_err(format!(
                "Invalid level: {n}. Expected a level between 1 and {}.",
                LEVELS.len()
            ))),
        }
    }

    /// The map stored in the file at `path` (or the built-in level it names, such as `"lvl6"`),
    /// with the given physics parameters (the default ones if `None`).
    ///
    /// Raises:
    ///     FileNotFoundError: if the file cannot be read.
    ///     ParsingError, UnsupportedFeatureError, ValueError: as the constructor.
    #[staticmethod]
    #[pyo3(signature = (path, params=None))]
    fn from_file(path: std::path::PathBuf, params: Option<PyPhysicsParams>) -> PyResult<Self> {
        Self::build(MapSpec::from_file(&path.to_string_lossy()), params)
    }

    /// The number of agents.
    #[getter]
    fn n_agents(&self) -> usize {
        self.world.n_agents()
    }

    /// The width of the world, in cells.
    #[getter]
    fn width(&self) -> usize {
        self.world.width()
    }

    /// The height of the world, in cells.
    #[getter]
    fn height(&self) -> usize {
        self.world.height()
    }

    /// The number of colours: the greatest colour of the agents and of the laser sources, plus 1.
    #[getter]
    fn n_colours(&self) -> usize {
        self.world.n_colours()
    }

    /// A copy of the physics parameters of the world: modifying it does not change the world.
    #[getter]
    fn params(&self) -> PyPhysicsParams {
        self.world.params().into()
    }

    /// The `(x, y)` position of the centre of each agent.
    #[getter]
    fn agents_positions(&self) -> Vec<(f64, f64)> {
        self.world
            .agents_positions()
            .into_iter()
            .map(|[x, y]| (x, y))
            .collect()
    }

    /// The cell `(i, j, 0)` that contains the centre of each agent.
    #[getter]
    fn agents_cells(&self) -> Vec<PyPosition> {
        positions(&self.world.agents_cells())
    }

    /// The colour of each agent.
    #[getter]
    fn agents_colours(&self) -> Vec<usize> {
        self.world.agents_colours()
    }

    /// Whether each agent is alive.
    #[getter]
    fn agents_alive(&self) -> Vec<bool> {
        self.world.agents_alive()
    }

    /// Whether each agent has reached an exit.
    #[getter]
    fn agents_arrived(&self) -> Vec<bool> {
        self.world.agents_arrived()
    }

    /// The cell of each gem.
    #[getter]
    fn gems_positions(&self) -> Vec<PyPosition> {
        positions(&self.world.gems_positions())
    }

    /// Whether each gem has been collected.
    #[getter]
    fn gems_collected(&self) -> Vec<bool> {
        self.world.gems_collected()
    }

    /// The cells of the exits.
    #[getter]
    fn exits_positions(&self) -> Vec<PyPosition> {
        positions(&self.world.exits_positions())
    }

    /// The possible start cells of each agent. At reset, every agent starts at the centre of one
    /// of its possible starts, drawn at random.
    #[getter]
    fn possible_starts(&self) -> Vec<Vec<PyPosition>> {
        self.world
            .possible_starts()
            .iter()
            .map(|starts| positions(starts))
            .collect()
    }

    /// The cells of the full (unblocked) beam of each laser source, in laser order and closest
    /// to the source first. They do not depend on the state.
    #[getter]
    fn beam_cells(&self) -> Vec<Vec<PyPosition>> {
        self.world
            .beam_cells()
            .iter()
            .map(|cells| positions(cells))
            .collect()
    }

    /// The `(cell, direction, colour, laser_id)` of each laser source, in laser order.
    ///
    /// @ai-generated
    #[getter]
    fn laser_sources(&self) -> Vec<(PyPosition, PyDirection, usize, usize)> {
        self.world
            .laser_sources()
            .iter()
            .map(|s| (s.position.into(), s.direction.into(), s.colour, s.laser_id))
            .collect()
    }

    /// The length of `state_vector()`.
    #[getter]
    fn state_size(&self) -> usize {
        self.world.state_size()
    }

    /// The length of `observation(agent)`.
    #[getter]
    fn observation_size(&self) -> usize {
        self.world.observation_size()
    }

    /// Put every agent at the centre of a random start, alive, still and not arrived, uncollect
    /// the gems and enable the laser sources.
    fn reset(&mut self) {
        self.world.reset();
    }

    /// Seed the random number generator that draws the starts at reset.
    ///
    /// Raises:
    ///     ValueError: if `seed` is negative or does not fit in 64 bits.
    ///
    /// @ai-generated
    fn seed(
        &mut self,
        #[gen_stub(override_type(type_repr = "builtins.int"))] seed: &Bound<'_, PyAny>,
    ) -> PyResult<()> {
        let value = seed.extract::<UnsignedInt>()?.0.ok_or_else(|| {
            PyValueError::new_err(format!(
                "Invalid seed {seed}: expected an integer in [0, 2**64)"
            ))
        })?;
        self.world.seed(value);
        Ok(())
    }

    /// Apply the joint action `actions` for `n_ticks` ticks, and stop early when the episode is
    /// over: an agent died, or every alive agent has arrived.
    ///
    /// Args:
    ///     actions: The `(ax, ay)` action of each agent, of shape `(n_agents, 2)`. Each component
    ///         is clipped to `[-1, 1]`, then the action is clipped to the unit disc.
    ///     n_ticks: The maximal number of ticks to run (at least 1).
    ///
    /// Raises:
    ///     ValueError: if `actions` does not have the shape `(n_agents, 2)`, or `n_ticks` is not
    ///         positive.
    ///     InvalidActionError: if an action is not finite.
    ///
    /// @ai-generated
    #[pyo3(signature = (actions, n_ticks=UnsignedInt(Some(1))))]
    fn step(
        &mut self,
        #[gen_stub(override_type(type_repr = "numpy.typing.ArrayLike", imports = ("numpy.typing",)))]
        actions: &Bound<'_, PyAny>,
        n_ticks: UnsignedInt,
    ) -> PyResult<PyStepOutcome> {
        let n_ticks = match n_ticks.as_usize() {
            Some(n) if n > 0 => n,
            _ => {
                return Err(PyValueError::new_err(
                    "n_ticks must be a positive integer that fits in 64 bits",
                ));
            }
        };
        let actions = Self::extract_actions(actions)?;
        let outcome = self
            .world
            .step(&actions, n_ticks)
            .map_err(continuous_error_to_pyexception)?;
        let (ticks, events) = outcome
            .events
            .iter()
            .map(|(t, e)| (*t, PyWorldEvent::from(e)))
            .unzip();
        Ok(PyStepOutcome {
            events,
            ticks,
            n_ticks: outcome.n_ticks,
        })
    }

    /// The current state of the world.
    fn get_state(&self) -> PyContinuousWorldState {
        self.world.get_state().into()
    }

    /// Restore a state. Hazards (voids, lasers) are only processed at the next tick.
    ///
    /// Raises:
    ///     InvalidWorldStateError: if the state does not fit the world (sizes), has a non-finite
    ///         value, or if an alive agent penetrates a wall, a laser source or the boundary, or
    ///         overlaps another alive agent. The world is then left unchanged.
    fn set_state(&mut self, state: PyContinuousWorldState) -> PyResult<()> {
        self.world
            .set_state(&(&state).into())
            .map_err(continuous_error_to_pyexception)
    }

    /// The `STATE` vector of the world, of length `state_size`: the block of every agent in id
    /// order, then one flag per gem (1 if collected) and one flag per laser source (1 if
    /// enabled). The block of an agent is `x / width, y / height, vx / max_speed,
    /// vy / max_speed, alive, arrived`, followed by its one-hot colour over `n_colours`.
    fn state_vector<'py>(&self, py: Python<'py>) -> Bound<'py, PyArray1<f32>> {
        PyArray1::from_vec(py, self.world.state_vector())
    }

    /// The `STATE` observation of `agent`, of length `observation_size`: its own block first,
    /// then the blocks of the other agents in increasing id order, whose `x` and `y` are relative
    /// to `agent` (`(x_k - x_agent) / width`, and likewise for `y`), then the gem and source
    /// flags. See `state_vector`.
    ///
    /// Raises:
    ///     IndexError: if `agent` is not the id of an agent (negative values included).
    ///
    /// @ai-generated
    fn observation<'py>(
        &self,
        py: Python<'py>,
        #[gen_stub(override_type(type_repr = "builtins.int"))] agent: &Bound<'_, PyAny>,
    ) -> PyResult<Bound<'py, PyArray1<f32>>> {
        let observation = agent
            .extract::<UnsignedInt>()?
            .as_usize()
            .and_then(|id| self.world.observation(id));
        match observation {
            Some(observation) => Ok(PyArray1::from_vec(py, observation)),
            None => Err(PyIndexError::new_err(format!(
                "Invalid agent id {agent}: the world has {} agents",
                self.world.n_agents()
            ))),
        }
    }

    /// An independent copy of the world, including its state and its random number generator.
    fn __deepcopy__(&self, _memo: &Bound<'_, PyDict>) -> Self {
        Self {
            world: self.world.clone(),
            map_str: self.map_str.clone(),
        }
    }

    /// The arguments given to `__new__` before `__setstate__` when unpickling.
    fn __getnewargs__<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyTuple>> {
        PyTuple::new(py, ["S0 X"])
    }

    /// Pickle the map, the parameters, the state and the random number generator.
    fn __getstate__(&self) -> PickledWorld {
        (
            self.map_str.clone(),
            self.world.params().into(),
            self.world.get_state().into(),
            self.world.rng_state().to_vec(),
        )
    }

    /// Restore a pickled world.
    ///
    /// @ai-generated
    fn __setstate__(&mut self, pickled: PickledWorld) -> PyResult<()> {
        let (map_str, params, state, rng_state) = pickled;
        let mut restored = Self::build(MapSpec::parse(&map_str), Some(params))?;
        restored
            .world
            .set_state(&(&state).into())
            .map_err(continuous_error_to_pyexception)?;
        let rng_state: [u8; RNG_STATE_SIZE] = rng_state
            .try_into()
            .map_err(|_| PyValueError::new_err("Invalid pickled random number generator state"))?;
        restored.world.set_rng_state(&rng_state);
        *self = restored;
        Ok(())
    }

    /// The size of the world and the current tick.
    ///
    /// @ai-generated
    fn __repr__(&self) -> String {
        format!(
            "ContinuousWorld(n_agents={}, width={}, height={}, tick={})",
            self.world.n_agents(),
            self.world.width(),
            self.world.height(),
            self.world.get_state().tick
        )
    }
}
