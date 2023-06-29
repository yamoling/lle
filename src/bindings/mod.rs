use pyo3::prelude::*;

use crate::{REWARD_AGENT_DIED, REWARD_AGENT_JUST_ARRIVED, REWARD_END_GAME, REWARD_GEM_COLLECTED};

mod pyaction;
mod pyagent;
mod pyworld;

#[pymodule]
pub fn lle(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_class::<pyworld::PyWorld>()?;
    m.add_class::<pyaction::PyAction>()?;
    m.add("REWARD_AGENT_DIED", REWARD_AGENT_DIED)?;
    m.add("REWARD_AGENT_JUST_ARRIVED", REWARD_AGENT_JUST_ARRIVED)?;
    m.add("REWARD_END_GAME", REWARD_END_GAME)?;
    m.add("REWARD_GEM_COLLECTED", REWARD_GEM_COLLECTED)?;
    // m.add_class::<crate::Action>()?;
    Ok(())
}
