use crate::solver::{ClauseGenerator, VarKey};

impl ClauseGenerator {
    /// Return literals asserting no cooperation: for every laser, every non-owner agent
    /// that could stand on a relevant beam tile is assumed not to be there.
    pub fn assume_no_cooperation(&mut self, t: usize) -> Vec<super::Literal> {
        self.ctx.update(t);
        let mut assumptions = Vec::with_capacity(self.ctx.laser_sources.len());
        for source in &self.ctx.laser_sources {
            let path = self.ctx.relevant_laser_tiles(source.laser_id, t);
            for agent in 0..self.ctx.n_agents {
                if agent == source.agent_id {
                    continue;
                }
                let positions = self.ctx.relevant_positions_for_agent(agent, t);
                for pos in path.intersection(&positions) {
                    let key = VarKey::agent(agent, pos, t);
                    let var = self
                        .pool
                        .get(&key)
                        .expect(&format!("Agent variable {key:?} does not exist."));
                    assumptions.push(-var);
                }
            }
        }
        assumptions
    }
}
