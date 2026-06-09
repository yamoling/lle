use std::collections::HashSet;

use crate::{Action, AgentId, Position, World};

use super::super::context::ConstraintContext;
use super::super::var_pool::{VarKey, VarPool};
use super::Clause;

/// Generates the SAT clauses for a single time step `t`, combining initialization,
/// movement and laser constraints.
pub struct ClauseGenerator {
    pub(super) ctx: ConstraintContext,
    pub(super) pool: VarPool,
    pub(super) exits: HashSet<Position>,
}

impl ClauseGenerator {
    pub fn new(world: &World, t_max: usize) -> Self {
        Self {
            exits: world.exits_positions().into_iter().collect(),
            ctx: ConstraintContext::new(world, t_max),
            pool: VarPool::new(),
        }
    }

    #[inline]
    pub fn decode_plan(&self, literals: &[i32], t_end: usize) -> Result<Vec<Vec<Action>>, String> {
        self.pool.decode_plan(literals, t_end)
    }

    #[inline]
    pub fn t_max(&self) -> usize {
        self.ctx.t_max
    }

    #[inline]
    pub fn solution_lower_bound(&self) -> usize {
        self.ctx.solution_lower_bound
    }

    pub fn exists(&self, key: &VarKey) -> bool {
        self.pool.exists(key)
    }

    /// The SAT literal for `mutual(a, b)` — "a and b mutually depend on each other" — or
    /// `None` if no such variable exists.
    pub fn mutual_lit(&self, a: AgentId, b: AgentId) -> Option<i32> {
        self.pool.get(&VarKey::mutual(a, b))
    }

    /// All clauses for time step `t`: initialization (t == 0), movement and laser constraints.
    pub fn generate(&mut self, t: usize) -> Vec<Clause> {
        self.ctx.update(t);
        let mut clauses = Vec::new();
        clauses.extend(self.initialization(t));
        clauses.extend(self.exactly_one_position(t));
        clauses.extend(self.time_wise_adjacency(t));
        clauses.extend(self.no_overlap(t));
        clauses.extend(self.no_following_conflict(t));
        clauses.extend(self.stays_on_exit(t));
        let (beam_clauses, active_lit) = self.beam_activation(t);
        clauses.extend(beam_clauses);
        clauses.extend(self.no_step_on_active_laser(t, &active_lit));
        clauses
    }

    /// Clauses asserting that, at the final time step `t`, every agent is on an exit.
    pub fn objective(&mut self, t: usize) -> Vec<Clause> {
        self.ctx.update(t);
        let mut clauses = Vec::with_capacity(self.ctx.n_agents);
        for agent in 0..self.ctx.n_agents {
            let reachable = self.ctx.relevant_positions(t, &[agent]);
            let positions: Vec<Position> = self
                .exits
                .iter()
                .copied()
                .filter(|p| reachable.contains(p))
                .collect();
            clauses.push(
                positions
                    .into_iter()
                    .map(|p| self.pool.agent(agent, p, t))
                    .collect(),
            );
        }
        clauses
    }
}
