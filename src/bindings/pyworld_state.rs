use super::pyposition::PyPosition;
use crate::core::WorldState;
use numpy::PyArray1;
use pyo3::{exceptions, prelude::*, pyclass::CompareOp, types::PyDict};
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pymethods};
use std::hash::{Hash, Hasher};
///
/// A state in the `World` is defined by:
///  - The position of each agent.
///  - Whether each gem has been collected.
///  - Whether each agent is alive.
/// ## Using `WorldState`s
/// ```python
/// from lle import WorldState, World
/// w = World("S0 . X")
/// w.reset()
/// s1 = w.get_state()
/// s2 = WorldState([(0, 1), [], [True]])
/// world.set_state(s2)
/// ```
/// ## Inheritance
/// To inherit from `WorldState`, it is required to override the `__new__` method such that you its signature
/// is compatible with `__init__`, i.e. it accepts the same leading arguments in the same order.
/// Additionally, the `__new__` method **must** call the `super()` constructor with the parameters of the parent class, as shown below.
/// ```python
/// class SubWorldState(WorldState):
///     def __init__(self, agents_positions: list[tuple[int, int]], gems_collected: list[bool], agents_alive: list[bool], x: int):
///         super().__init__(agents_positions, gems_collected, agents_alive)
///         self.x = x
///     def __new__(cls, agents_positions: list[tuple[int, int]], gems_collected: list[bool], agents_alive: list[bool], *args, **kwargs):
///         instance = super().__new__(cls, agents_positions, gems_collected, agents_alive)
///         return instance
/// ```
#[gen_stub_pyclass]
#[pyclass(name = "WorldState", module = "lle", subclass)]
#[derive(Clone, Hash)]
pub struct PyWorldState {
    /// The position of each agent.
    #[pyo3(get, set)]
    agents_positions: Vec<PyPosition>,
    /// The collection status of each gem.
    #[pyo3(get, set)]
    gems_collected: Vec<bool>,
    /// The status of each agent.
    #[pyo3(get, set)]
    agents_alive: Vec<bool>,
}

#[gen_stub_pymethods]
#[pymethods]
impl PyWorldState {
    #[new]
    #[pyo3(signature = (agents_positions, gems_collected, agents_alive=None))]
    pub fn new(
        agents_positions: Vec<(usize, usize)>,
        gems_collected: Vec<bool>,
        agents_alive: Option<Vec<bool>>,
    ) -> Self {
        let agents_alive = agents_alive.unwrap_or_else(|| vec![true; agents_positions.len()]);
        Self {
            agents_positions,
            gems_collected,
            agents_alive,
        }
    }

    #[pyo3(signature = (agents_positions, gems_collected, agents_alive=None))]
    fn __init__(
        &mut self,
        agents_positions: Vec<(usize, usize)>,
        gems_collected: Vec<bool>,
        agents_alive: Option<Vec<bool>>,
    ) {
        let agents_alive = agents_alive.unwrap_or_else(|| vec![true; agents_positions.len()]);
        self.agents_positions = agents_positions;
        self.gems_collected = gems_collected;
        self.agents_alive = agents_alive;
    }

    fn as_array(&self, py: Python) -> PyObject {
        let len = self.agents_positions.len() * 3 + self.gems_collected.len();
        let mut res = Vec::with_capacity(len);
        for (i, j) in &self.agents_positions {
            res.push(*i as f32);
            res.push(*j as f32);
        }
        for is_collected in &self.gems_collected {
            if *is_collected {
                res.push(1.0);
            } else {
                res.push(0.0);
            }
        }
        for alive in &self.agents_alive {
            if *alive {
                res.push(1.0);
            } else {
                res.push(0.0);
            }
        }
        PyArray1::from_vec_bound(py, res).into()
    }

    #[staticmethod]
    fn from_array(array: Vec<f32>, n_agents: usize, n_gems: usize) -> PyResult<Self> {
        let expected_len = n_agents * 3 + n_gems;
        if array.len() != expected_len {
            return Err(exceptions::PyValueError::new_err(format!(
                "The array must have a length of {expected_len}.",
            )));
        }

        let mut agents_positions = Vec::with_capacity(n_agents);
        for i in 0..n_agents {
            agents_positions.push((array[i * 2] as usize, array[i * 2 + 1] as usize));
        }
        let mut gems_collected = Vec::with_capacity(n_gems);
        for i in 0..n_gems {
            let is_collected = array[n_agents * 2 + i] == 1.0;
            gems_collected.push(is_collected);
        }
        let mut agents_alive = Vec::with_capacity(n_agents);
        for i in 0..n_agents {
            let is_alive = array[n_agents * 2 + n_gems + i] == 1.0;
            agents_alive.push(is_alive);
        }

        Ok(Self {
            agents_positions,
            gems_collected,
            agents_alive,
        })
    }

    fn __deepcopy__(&self, _memo: &Bound<'_, PyDict>) -> Self {
        self.clone()
    }

    fn __getstate__(&self) -> PyResult<(Vec<bool>, Vec<PyPosition>, Vec<bool>)> {
        Ok((
            self.gems_collected.clone(),
            self.agents_positions.clone(),
            self.agents_alive.clone(),
        ))
    }

    fn __setstate__(&mut self, state: (Vec<bool>, Vec<PyPosition>, Vec<bool>)) -> PyResult<()> {
        let (gems_collected, agents_positions, agents_alive) = state;
        self.gems_collected = gems_collected;
        self.agents_positions = agents_positions;
        self.agents_alive = agents_alive;
        Ok(())
    }

    pub fn __getnewargs__(&self) -> (Vec<PyPosition>, Vec<bool>, Option<Vec<bool>>) {
        (vec![], vec![], None)
    }

    fn __repr__(&self) -> String {
        format!(
            "WorldState(agents_positions={:?}, gems_collected={:?}, agents_alive={:?})",
            self.agents_positions, self.gems_collected, self.agents_alive
        )
    }

    fn __hash__(&self) -> u64 {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        self.hash(&mut hasher);
        hasher.finish()
    }

    fn __richcmp__(&self, other: &Self, cmp: CompareOp) -> PyResult<bool> {
        let eq = self.agents_positions == other.agents_positions
            && self.gems_collected == other.gems_collected
            && self.agents_alive == other.agents_alive;
        match cmp {
            CompareOp::Eq => Ok(eq),
            CompareOp::Ne => Ok(!eq),
            other => Err(exceptions::PyArithmeticError::new_err(format!(
                "Unsupported comparison: {other:?}"
            ))),
        }
    }
}

impl From<PyWorldState> for WorldState {
    fn from(val: PyWorldState) -> Self {
        WorldState {
            agents_positions: val.agents_positions.into_iter().map(Into::into).collect(),
            gems_collected: val.gems_collected,
            agents_alive: val.agents_alive,
        }
    }
}

impl Into<PyWorldState> for WorldState {
    fn into(self) -> PyWorldState {
        PyWorldState {
            agents_positions: self.agents_positions.into_iter().map(Into::into).collect(),
            gems_collected: self.gems_collected,
            agents_alive: self.agents_alive,
        }
    }
}
