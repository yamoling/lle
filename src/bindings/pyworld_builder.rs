use std::collections::{HashMap, HashSet};

use pyo3::{exceptions::PyValueError, prelude::*};
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pymethods};

use crate::{tiles::LaserId, AgentId, Position, World};

use super::{
    pydirection::PyDirection, pyexceptions::parse_error_to_exception, pyposition::PyPosition,
    pyworld::PyWorld,
};

#[gen_stub_pyclass]
#[pyclass(name = "WorldBuilder", module = "lle")]
pub struct PyWorldBuilder {
    state: Vec<Vec<String>>,
    #[pyo3(get)]
    width: usize,
    #[pyo3(get)]
    height: usize,
    #[pyo3(get)]
    n_agents: usize,
    n_lasers: LaserId,
    #[pyo3(get)]
    start_positions: HashMap<AgentId, PyPosition>,
    #[pyo3(get)]
    exit_positions: HashSet<PyPosition>,
    #[pyo3(get)]
    available_positions: HashSet<PyPosition>,
}

impl PyWorldBuilder {
    fn position_check(&self, pos: Position) -> PyResult<Position> {
        let (i, j) = pos.as_ij();
        if i >= self.height || j >= self.width {
            return Err(PyValueError::new_err("Position out of bounds"));
        }
        if !self.available_positions.contains(&pos.into()) {
            return Err(PyValueError::new_err("Position already occupied"));
        }
        Ok(pos)
    }
}

#[gen_stub_pymethods]
#[pymethods]
impl PyWorldBuilder {
    #[new]
    pub fn new(width: usize, height: usize, n_agents: usize) -> Self {
        PyWorldBuilder {
            state: vec![vec![".".into(); width]; height],
            width,
            height,
            n_agents,
            n_lasers: 0,
            start_positions: HashMap::new(),
            exit_positions: HashSet::new(),
            available_positions: (0..height)
                .flat_map(|i| (0..width).map(move |j| (i, j)))
                .collect(),
        }
    }

    fn build(&self) -> PyResult<PyWorld> {
        // Join every row into a single string separated by newlines and white spaces
        let world_str = self.world_str();
        let world = match World::try_from(world_str) {
            Ok(world) => world,
            Err(err) => return Err(parse_error_to_exception(err)),
        };
        Ok(PyWorld::from(world))
    }

    fn can_build(&self) -> bool {
        self.n_agents == self.start_positions.len() && self.exit_positions.len() >= self.n_agents
    }

    fn world_str(&self) -> String {
        self.state
            .iter()
            .map(|row| row.join(" "))
            .collect::<Vec<String>>()
            .join("\n")
    }

    fn set_start(&mut self, pos: Position, agent_num: usize) -> PyResult<()> {
        if agent_num >= self.n_agents {
            return Err(PyValueError::new_err("Agent number out of bounds"));
        }
        let (i, j) = self.position_check(pos)?;
        if let Some((prev_i, prev_j)) = self.start_positions.get(&agent_num) {
            self.state[*prev_i][*prev_j].clear();
            self.available_positions.insert((*prev_i, *prev_j));
        }
        self.start_positions.insert(agent_num, pos);
        self.available_positions.remove(&(i, j));
        self.state[i][j] = format!("S{agent_num}");
        Ok(())
    }

    fn add_laser_source(
        &mut self,
        pos: Position,
        agent_id: AgentId,
        direction: PyDirection,
    ) -> PyResult<()> {
        let (i, j) = self.position_check(pos)?;
        let s: &str = direction.into();
        self.state[i][j] = format!("L{agent_id}{s}");
        self.available_positions.remove(&(i, j));
        Ok(())
    }

    fn add_wall(&mut self, pos: Position) -> PyResult<()> {
        let (i, j) = self.position_check(pos)?;
        self.state[i][j] = "@".into();
        self.available_positions.remove(&(i, j));
        Ok(())
    }

    fn add_exit(&mut self, pos: Position) -> PyResult<()> {
        let (i, j) = self.position_check(pos)?;
        self.state[i][j] = "X".into();
        self.exit_positions.insert((i, j));
        self.available_positions.remove(&(i, j));
        Ok(())
    }

    fn add_gem(&mut self, pos: Position) -> PyResult<()> {
        let (i, j) = self.position_check(pos)?;
        self.state[i][j] = "G".into();
        self.available_positions.remove(&(i, j));
        Ok(())
    }

    fn reset(&mut self) {
        self.n_lasers = 0;
        self.state = vec![vec![".".into(); self.width]; self.height];
        self.start_positions.clear();
        self.exit_positions.clear();
        self.available_positions = (0..self.height)
            .flat_map(|i| (0..self.width).map(move |j| (i, j)))
            .collect();
    }

    fn clear(&mut self, pos: Position) -> PyResult<()> {
        let (i, j) = pos;
        if i >= self.height || j >= self.width {
            return Err(PyValueError::new_err("Position out of bounds"));
        }
        self.state[i][j] = ".".into();
        self.available_positions.insert((i, j));
        Ok(())
    }
}
