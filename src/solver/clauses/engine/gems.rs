use crate::solver::{Clause, clauses::ClauseEngine};

impl ClauseEngine {
    /// Cache occupancy literals for gem/time combinations that have not been visited yet.
    ///
    /// @ai-generated
    fn cache_gem_literal_chunks_before(&mut self, t: usize) {
        for tau in self.next_uncached_gem_time..t {
            for (gem_index, &gem_pos) in self.gems.iter().enumerate() {
                let mut literals = Vec::new();
                for agent in 0..self.ctx.n_agents {
                    if self
                        .ctx
                        .relevant_positions_for_agent(agent, tau)
                        .contains(&gem_pos)
                    {
                        literals.push(self.pool.agent(agent, gem_pos, tau));
                    }
                }
                self.gem_literal_chunks[gem_index].push(literals);
            }
        }
        self.next_uncached_gem_time = self.next_uncached_gem_time.max(t);
    }

    /// Express gem collection as one occupancy disjunction per gem for the queried horizon.
    ///
    /// The cached chunks retain the original time-major, agent-minor literal order and allow
    /// objectives for shorter horizons to reuse the corresponding prefix.
    ///
    /// @ai-generated
    pub fn gems_must_be_collected(&mut self, t: usize) -> Vec<Clause> {
        self.ctx.update(t);
        self.cache_gem_literal_chunks_before(t);
        self.gem_literal_chunks
            .iter()
            .map(|chunks| {
                chunks
                    .iter()
                    .take(t.saturating_sub(1))
                    .flatten()
                    .copied()
                    .collect()
            })
            .collect()
    }
}
