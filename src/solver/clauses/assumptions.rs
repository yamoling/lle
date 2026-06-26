use super::engine::ClauseEngine;
use super::{Literal, VarKey};

impl ClauseEngine {
    /// Return literals asserting no cooperation at exactly time `t`: for every laser, every
    /// non-owner agent that could stand on a relevant beam tile is assumed not to be there.
    pub(crate) fn assume_no_cooperation_at(&mut self, t: usize) -> Vec<Literal> {
        self.ctx.update(t);
        let mut assumptions = Vec::new();
        for source in &self.ctx.laser_sources {
            let path = self.ctx.relevant_laser_tiles(source.laser_id, t);
            for agent in 0..self.ctx.n_agents {
                if agent == source.agent_id {
                    continue;
                }
                let positions = self.ctx.relevant_positions_for_agent(agent, t);
                for pos in path.intersection(positions) {
                    let key = VarKey::agent(agent, pos, t);
                    let var = self
                        .pool
                        .get(&key)
                        .unwrap_or_else(|| panic!("Agent variable {key:?} does not exist."));
                    assumptions.push(-var);
                }
            }
        }
        assumptions
    }
}

#[cfg(test)]
#[path = "../../unit_tests/test_assumptions.rs"]
mod tests;
