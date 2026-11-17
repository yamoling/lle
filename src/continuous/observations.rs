//! The `STATE` observation: flat `f32` vectors describing the agents and the dynamic flags of the
//! world.
//!
//! - **Agent block** (`6 + n_colours` values): `x / width`, `y / height`, `vx / max_speed`,
//!   `vy / max_speed`, `alive`, `arrived`, then the one-hot colour of the agent. The velocity is
//!   that of the state: the displacement of the last tick divided by `dt`.
//! - **Global block**: one flag per gem (1 when collected), then one flag per laser source (1
//!   when enabled), in laser order.
//!
//! The state vector is every agent block in id order, with absolute coordinates, followed by
//! the global block. The observation of an agent puts its own block first (absolute), then the
//! blocks of the other agents in increasing id order, where only `x` and `y` are relative to the
//! observer, and finally the global block.
use super::ContinuousWorld;
use crate::shared::AgentId;

/// The number of values of an agent block before its one-hot colour.
const AGENT_FEATURES: usize = 6;

impl ContinuousWorld {
    /// The number of values describing one agent: 6 features and a one-hot colour.
    pub fn agent_block_size(&self) -> usize {
        AGENT_FEATURES + self.n_colours()
    }

    /// The length of [`ContinuousWorld::state_vector`].
    pub fn state_size(&self) -> usize {
        self.n_agents() * self.agent_block_size() + self.gems.len() + self.beams.len()
    }

    /// The length of [`ContinuousWorld::observation`], which is that of the state vector.
    pub fn observation_size(&self) -> usize {
        self.state_size()
    }

    /// The state of the world as a flat vector: every agent block in id order, with absolute
    /// coordinates, then the global block (see the module documentation).
    ///
    /// @ai-generated
    pub fn state_vector(&self) -> Vec<f32> {
        let mut res = Vec::with_capacity(self.state_size());
        for agent in 0..self.n_agents() {
            self.push_agent_block(&mut res, agent, [0.0, 0.0]);
        }
        self.push_global_block(&mut res);
        res
    }

    /// The observation of `agent`: its own block first, with absolute coordinates, then the
    /// blocks of the other agents in increasing id order, whose `x` and `y` are relative to
    /// `agent`, then the global block (see the module documentation).
    ///
    /// `None` when `agent` is not the id of an agent of the world.
    ///
    /// @ai-generated
    pub fn observation(&self, agent: AgentId) -> Option<Vec<f32>> {
        if agent >= self.n_agents() {
            return None;
        }
        let mut res = Vec::with_capacity(self.observation_size());
        self.push_agent_block(&mut res, agent, [0.0, 0.0]);
        let origin = self.state.positions[agent];
        for other in (0..self.n_agents()).filter(|&other| other != agent) {
            self.push_agent_block(&mut res, other, origin);
        }
        self.push_global_block(&mut res);
        Some(res)
    }

    /// Append the block of `agent` to `res`, with its position relative to `origin`.
    ///
    /// @ai-generated
    fn push_agent_block(&self, res: &mut Vec<f32>, agent: AgentId, origin: [f64; 2]) {
        let [x, y] = self.state.positions[agent];
        let [vx, vy] = self.state.velocities[agent];
        let max_speed = self.params.max_speed;
        res.extend([
            ((x - origin[0]) / self.width as f64) as f32,
            ((y - origin[1]) / self.height as f64) as f32,
            (vx / max_speed) as f32,
            (vy / max_speed) as f32,
            flag(self.state.agents_alive[agent]),
            flag(self.state.agents_arrived[agent]),
        ]);
        let colour = self.colours[agent];
        res.extend((0..self.n_colours()).map(|c| flag(c == colour)));
    }

    /// Append the global block to `res`: the gem flags, then the source flags.
    fn push_global_block(&self, res: &mut Vec<f32>) {
        res.extend(self.state.gems_collected.iter().map(|&c| flag(c)));
        res.extend(self.state.sources_enabled.iter().map(|&e| flag(e)));
    }
}

fn flag(value: bool) -> f32 {
    if value { 1.0 } else { 0.0 }
}

#[cfg(test)]
#[path = "../unit_tests/test_continuous_observations.rs"]
mod tests;
