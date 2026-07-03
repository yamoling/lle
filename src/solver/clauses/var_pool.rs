use std::collections::HashMap;

use crate::{Action, AgentId, Position, solver::errors::SolverError};

/// Semantic key for a SAT variable.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum VarKey {
    /// Whether the specified agent is located at `pos` at time step `t`.
    Agent {
        agent_id: AgentId,
        pos: Position,
        t: usize,
    },
    /// Whether (laser_id, i, j) is active at time step t
    Laser {
        laser_id: AgentId,
        pos: Position,
        t: usize,
    },
    /// Whether `helper` is helping `beneficiary` at time step `t`.
    Help {
        helper: AgentId,
        beneficiary: AgentId,
        t: usize,
    },
    /// Shorthand for "there exists a time step `t` at which `beneficiary` is the beneficiary of `help(h, b, t)`"
    IsHelped {
        beneficiary: AgentId,
    },
    Asymmetric,
    /// Auxiliary variable used internally by cardinality encodings; carries a unique counter.
    Aux(i32),
}

impl VarKey {
    #[inline]
    pub fn agent(id: AgentId, pos: Position, t: usize) -> Self {
        VarKey::Agent {
            agent_id: id,
            pos,
            t,
        }
    }

    #[inline]
    pub fn laser(id: AgentId, pos: Position, t: usize) -> Self {
        VarKey::Laser {
            laser_id: id,
            pos,
            t,
        }
    }
}

/// Lazily assigns sequential positive integer ids to semantic variable keys,
/// keeping the SAT variable space dense and small (mirrors `pysat.formula.IDPool`).
#[derive(Default)]
pub struct VarPool {
    ids: HashMap<VarKey, i32>,
    keys: Vec<VarKey>,
}

impl VarPool {
    pub fn new() -> Self {
        Self::default()
    }

    fn id(&mut self, key: VarKey) -> i32 {
        if let Some(&id) = self.ids.get(&key) {
            return id;
        }
        let id = self.next_id();
        self.ids.insert(key, id);
        self.keys.push(key);
        id
    }

    pub fn agent(&mut self, agent_id: AgentId, pos: Position, t: usize) -> i32 {
        self.id(VarKey::Agent { agent_id, pos, t })
    }

    pub fn laser(&mut self, laser_id: usize, pos: Position, t: usize) -> i32 {
        self.id(VarKey::Laser { laser_id, pos, t })
    }

    pub fn help(&mut self, helper: AgentId, beneficiary: AgentId, t: usize) -> i32 {
        self.id(VarKey::Help {
            helper,
            beneficiary,
            t,
        })
    }

    pub fn asymmetric(&mut self) -> i32 {
        self.id(VarKey::Asymmetric)
    }

    /// Variable id already assigned to `key`, or `None` if it was never created.
    ///
    /// Unlike the factory methods above, this never *creates* a variable, so it is safe to use
    /// when probing whether a (possibly non-existent) cooperation variable should be constrained.
    pub fn get(&self, key: &VarKey) -> Option<i32> {
        self.ids.get(key).copied()
    }

    fn next_id(&self) -> i32 {
        // ids start at 1, as required by SAT solvers
        self.ids.len() as i32 + 1
    }

    pub fn aux(&mut self) -> i32 {
        self.id(VarKey::Aux(self.next_id()))
    }

    pub fn key(&self, id: i32) -> Option<VarKey> {
        if id <= 0 {
            return None;
        }
        self.keys.get((id - 1) as usize).copied()
    }

    pub fn exists(&self, key: &VarKey) -> bool {
        self.ids.contains_key(key)
    }

    pub fn n_vars(&self) -> usize {
        self.ids.len()
    }

    /// Decode a SAT model (list of signed literals) into a joint action plan of length `t_end`.
    ///
    /// Returns [`SolverError::MissingPosition`] if the model does not pin down every agent's
    /// position at each step `0..=t_end`, and [`SolverError::InvalidTrajectory`] if two
    /// consecutive positions are not connected by a single action.
    pub fn decode_plan(
        &self,
        literals: &[i32],
        t_end: usize,
    ) -> Result<Vec<Vec<Action>>, SolverError> {
        let mut positions: HashMap<usize, HashMap<usize, Position>> = HashMap::new();
        for &lit in literals {
            if lit <= 0 {
                continue;
            }
            if let Some(VarKey::Agent { agent_id, pos, t }) = self.key(lit) {
                positions.entry(agent_id).or_default().insert(t, pos);
            }
        }
        let mut agent_ids: Vec<usize> = positions.keys().copied().collect();
        agent_ids.sort_unstable();

        let position_at = |agent: usize, t: usize| -> Result<Position, SolverError> {
            positions[&agent]
                .get(&t)
                .copied()
                .ok_or(SolverError::MissingPosition { agent, t })
        };

        let mut plan = Vec::with_capacity(t_end);
        for t in 0..t_end {
            let mut row = Vec::with_capacity(agent_ids.len());
            for &agent in &agent_ids {
                let (prev, current) = (position_at(agent, t)?, position_at(agent, t + 1)?);
                let action = (current - prev).map_err(|_| SolverError::InvalidTrajectory {
                    prev_pos: prev,
                    current_pos: current,
                    agent,
                    index: t + 1,
                })?;
                row.push(action);
            }
            plan.push(row);
        }
        Ok(plan)
    }
}

#[cfg(test)]
#[path = "../../unit_tests/test_var_pool.rs"]
mod tests;
