//! Pure-Rust port of the SAT constraint generation used by `lle.solver.solve`.
//!
//! This module reproduces (and replaces) the Python `ConstraintContext`,
//! `VariableFactory` and `Initialization/Movement/Laser/Objective` constraint
//! generators. The SAT solving itself (Minisat22 via `pysat`) remains in Python;
//! this module only produces the CNF clauses and decodes solver models back into
//! plans.

mod clauses;
mod context;
mod var_pool;

pub use clauses::{Clause, ClauseGenerator};
pub use context::{ConstraintContext, Pos};
pub use var_pool::VarKey;

use std::collections::HashMap;

use crate::{Action, World};

/// High level entry point: builds the constraint context once and exposes incremental
/// clause generation plus model decoding.
pub struct ConstraintGenerator {
    generator: ClauseGenerator,
}

impl ConstraintGenerator {
    pub fn new(world: &World, t_max: usize) -> Self {
        let ctx = ConstraintContext::new(world, t_max);
        ConstraintGenerator { generator: ClauseGenerator::new(ctx) }
    }

    pub fn solution_lower_bound(&self) -> usize {
        self.generator.ctx().solution_lower_bound
    }

    pub fn t_max(&self) -> usize {
        self.generator.ctx().t_max
    }

    pub fn generate(&mut self, t: usize) -> Vec<Clause> {
        self.generator.generate(t)
    }

    pub fn objective(&mut self, t: usize) -> Vec<Clause> {
        self.generator.objective(t)
    }

    pub fn no_blocking_clauses(&mut self, t: usize) -> Vec<Clause> {
        self.generator.no_blocking_clauses(t)
    }

    /// Decode a SAT model (list of signed literals) into a joint action plan of length `t_end`.
    pub fn decode_plan(&self, model: &[i32], t_end: usize) -> Result<Vec<Vec<Action>>, String> {
        let mut positions: HashMap<usize, HashMap<usize, Pos>> = HashMap::new();
        for &lit in model {
            if lit <= 0 {
                continue;
            }
            if let Some(VarKey::Agent(agent, i, j, t)) = self.generator.pool.key(lit) {
                positions.entry(agent).or_default().insert(t, (i, j));
            }
        }
        let mut agent_ids: Vec<usize> = positions.keys().copied().collect();
        agent_ids.sort_unstable();

        let mut plan = Vec::with_capacity(t_end);
        for t in 0..t_end {
            let mut row = Vec::with_capacity(agent_ids.len());
            for &agent in &agent_ids {
                let (y1, x1) = positions[&agent][&t];
                let (y2, x2) = positions[&agent][&(t + 1)];
                let (dx, dy) = (x2 as i64 - x1 as i64, y2 as i64 - y1 as i64);
                let action = match (dx, dy) {
                    (0, 0) => Action::Stay,
                    (0, -1) => Action::North,
                    (0, 1) => Action::South,
                    (1, 0) => Action::East,
                    (-1, 0) => Action::West,
                    _ => {
                        return Err(format!(
                            "Invalid movement for agent {agent} at t={t}->{}: delta=({dx}, {dy})",
                            t + 1
                        ))
                    }
                };
                row.push(action);
            }
            plan.push(row);
        }
        Ok(plan)
    }
}
