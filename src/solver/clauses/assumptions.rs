use super::{ClauseGenerator, Literal, VarKey};

impl ClauseGenerator {
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

    /// Return all no-cooperation assumptions for steps `0..=t`.
    pub(crate) fn assume_no_cooperation_until(&mut self, t: usize) -> Vec<Literal> {
        self.ensure_domain(t, false);
        let start = self.no_cooperation_generated_until.map_or(0, |u| u + 1);
        for tt in start..=t {
            self.no_cooperation_assumption_buffer[tt] = self.assume_no_cooperation_at(tt);
        }
        if start <= t {
            self.no_cooperation_generated_until = Some(t);
        }
        self.no_cooperation_assumption_buffer[..=t]
            .iter()
            .flatten()
            .copied()
            .collect()
    }
}

#[cfg(test)]
#[path = "../../unit_tests/test_assumptions.rs"]
mod tests;
