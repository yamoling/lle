use crate::continuous::ContinuousWorldState;
use pyo3::{prelude::*, types::PyDict};
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pymethods};

type PickledState = (
    Vec<(f64, f64)>,
    Vec<(f64, f64)>,
    Vec<bool>,
    Vec<bool>,
    Vec<bool>,
    Vec<bool>,
    u64,
);

/// The complete dynamic state of a `ContinuousWorld`: restoring it with `set_state` makes the
/// world replay exactly the same ticks.
///
/// Positions and velocities are `(x, y)` pairs: `x` grows eastwards (columns) and `y`
/// southwards (rows). The centre of the cell `(i, j)` is `(j + 0.5, i + 0.5)`.
///
/// Two states are equal when all their values are exactly equal.
#[gen_stub_pyclass]
#[pyclass(
    name = "ContinuousWorldState",
    module = "lle.continuous.world",
    eq,
    from_py_object
)]
#[derive(Clone, Debug, PartialEq)]
pub struct PyContinuousWorldState {
    /// The `(x, y)` position of the centre of each agent.
    #[pyo3(get, set)]
    positions: Vec<(f64, f64)>,
    /// The `(vx, vy)` velocity of each agent, in cells per second.
    #[pyo3(get, set)]
    velocities: Vec<(f64, f64)>,
    /// Whether each gem has been collected.
    #[pyo3(get, set)]
    gems_collected: Vec<bool>,
    /// Whether each agent is alive.
    #[pyo3(get, set)]
    agents_alive: Vec<bool>,
    /// Whether each agent has reached an exit.
    #[pyo3(get, set)]
    agents_arrived: Vec<bool>,
    /// Whether each laser source is enabled, in laser order.
    #[pyo3(get, set)]
    sources_enabled: Vec<bool>,
    /// The number of ticks since the last reset.
    #[pyo3(get, set)]
    tick: u64,
}

#[gen_stub_pymethods]
#[pymethods]
impl PyContinuousWorldState {
    #[new]
    pub fn new(
        positions: Vec<(f64, f64)>,
        velocities: Vec<(f64, f64)>,
        gems_collected: Vec<bool>,
        agents_alive: Vec<bool>,
        agents_arrived: Vec<bool>,
        sources_enabled: Vec<bool>,
        tick: u64,
    ) -> Self {
        Self {
            positions,
            velocities,
            gems_collected,
            agents_alive,
            agents_arrived,
            sources_enabled,
            tick,
        }
    }

    fn __repr__(&self) -> String {
        format!(
            "ContinuousWorldState(positions={:?}, velocities={:?}, gems_collected={:?}, agents_alive={:?}, agents_arrived={:?}, sources_enabled={:?}, tick={})",
            self.positions,
            self.velocities,
            self.gems_collected,
            self.agents_alive,
            self.agents_arrived,
            self.sources_enabled,
            self.tick
        )
    }

    fn __deepcopy__(&self, _memo: &Bound<'_, PyDict>) -> Self {
        self.clone()
    }

    fn __copy__(&self) -> Self {
        self.clone()
    }

    /// The arguments of the constructor, which is enough to pickle the state.
    fn __getnewargs__(&self) -> PickledState {
        (
            self.positions.clone(),
            self.velocities.clone(),
            self.gems_collected.clone(),
            self.agents_alive.clone(),
            self.agents_arrived.clone(),
            self.sources_enabled.clone(),
            self.tick,
        )
    }
}

impl From<ContinuousWorldState> for PyContinuousWorldState {
    fn from(state: ContinuousWorldState) -> Self {
        let pairs = |v: Vec<[f64; 2]>| v.into_iter().map(|[x, y]| (x, y)).collect();
        Self {
            positions: pairs(state.positions),
            velocities: pairs(state.velocities),
            gems_collected: state.gems_collected,
            agents_alive: state.agents_alive,
            agents_arrived: state.agents_arrived,
            sources_enabled: state.sources_enabled,
            tick: state.tick,
        }
    }
}

impl From<&PyContinuousWorldState> for ContinuousWorldState {
    fn from(state: &PyContinuousWorldState) -> Self {
        let arrays = |v: &[(f64, f64)]| v.iter().map(|&(x, y)| [x, y]).collect();
        Self {
            positions: arrays(&state.positions),
            velocities: arrays(&state.velocities),
            gems_collected: state.gems_collected.clone(),
            agents_alive: state.agents_alive.clone(),
            agents_arrived: state.agents_arrived.clone(),
            sources_enabled: state.sources_enabled.clone(),
            tick: state.tick,
        }
    }
}
