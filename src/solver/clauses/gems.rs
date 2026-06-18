use itertools::Itertools;

use crate::solver::{Clause, ClauseGenerator};

impl ClauseGenerator {
    /// In comparison to other clauses, gem collection should be expressed at a single time-step.
    /// The most intuitive formulation of the clause is simply a large OR-clause that checks if
    /// at some point in time, an agent is standing on the gem tile.
    pub(super) fn gems_must_be_collected(&mut self, t: usize) -> Vec<Clause> {
        self.ctx.update(t);
        self.gems
            .iter()
            .map(|gem_pos| {
                // We exclude tau=0 and tau=t because a gem cannot be on start or exit tiles
                (1..t)
                    .cartesian_product(0..self.ctx.n_agents)
                    .filter(|&(tau, agent)| {
                        self.ctx
                            .relevant_positions_for_agent(agent, tau)
                            .contains(gem_pos)
                    })
                    .map(|(tau, agent)| self.pool.agent(agent, *gem_pos, tau))
                    .collect()
            })
            .collect()
    }
}
