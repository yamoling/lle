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
    /// Whether `helper` has helped `beneficiary` at any time step ≤ `t` (a monotone temporal
    /// prefix-OR over the per-step help events).
    HasHelpedByTime {
        helper: AgentId,
        beneficiary: AgentId,
        t: usize,
    },
    /// Whether agents `a` and `b` mutually depend on each other (canonical: `a < b`).
    Mutual { a: AgentId, b: AgentId },
    /// Whether the concrete help event `helper -> beneficiary` at `pos` and time `t` is asymmetric,
    /// i.e. `helper` is not helped by any other agent by the solve horizon. The variable is used as
    /// a forbid-by-assumption literal in `no-asymmetric` mode.
    Asymmetric {
        helper: AgentId,
        beneficiary: AgentId,
        pos: Position,
        t: usize,
    },
    /// Progress for chain `chain_id` of forbidden length `length`: its first `step` edges have
    /// fired with non-decreasing timestamps, the `step`-th edge firing at some time ≤ `t`.
    /// Only created for `step ≥ 2`; the first edge is expressed directly by [`HasHelpedByTime`].
    ChainProgress {
        length: usize,
        chain_id: u32,
        step: u8,
        t: usize,
    },
    /// Whether chain `chain_id` of forbidden length `length` has been fully realized.
    ChainRealized { length: usize, chain_id: u32 },
    /// Progress for cycle rotation `cycle_id` of forbidden order `order`: its first `step` edges
    /// have fired with non-decreasing timestamps, the `step`-th edge firing at some time ≤ `t`.
    /// Only created for `step ≥ 2`; the first edge is expressed directly by [`HasHelpedByTime`].
    CycleProgress {
        order: usize,
        cycle_id: u32,
        step: u8,
        t: usize,
    },
    /// Whether cycle rotation `cycle_id` of forbidden order `order` has been fully realized.
    CycleRealized { order: usize, cycle_id: u32 },
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

    #[inline]
    pub fn has_helped_by_time(helper: AgentId, beneficiary: AgentId, t: usize) -> Self {
        VarKey::HasHelpedByTime {
            helper,
            beneficiary,
            t,
        }
    }

    /// Canonical (min < max) mutual-dependency key for the unordered pair `{a, b}`.
    #[inline]
    pub fn mutual(a: AgentId, b: AgentId) -> Self {
        let (lo, hi) = if a < b { (a, b) } else { (b, a) };
        VarKey::Mutual { a: lo, b: hi }
    }

    #[inline]
    pub fn asymmetric(helper: AgentId, beneficiary: AgentId, pos: Position, t: usize) -> Self {
        VarKey::Asymmetric {
            helper,
            beneficiary,
            pos,
            t,
        }
    }

    #[inline]
    pub fn chain_progress(length: usize, chain_id: u32, step: u8, t: usize) -> Self {
        VarKey::ChainProgress {
            length,
            chain_id,
            step,
            t,
        }
    }

    #[inline]
    pub fn chain_realized(length: usize, chain_id: u32) -> Self {
        VarKey::ChainRealized { length, chain_id }
    }

    #[inline]
    pub fn cycle_progress(order: usize, cycle_id: u32, step: u8, t: usize) -> Self {
        VarKey::CycleProgress {
            order,
            cycle_id,
            step,
            t,
        }
    }

    #[inline]
    pub fn cycle_realized(order: usize, cycle_id: u32) -> Self {
        VarKey::CycleRealized { order, cycle_id }
    }

    #[inline]
    pub fn aux(id: i32) -> Self {
        VarKey::Aux(id)
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

    /// Indicator "`a` and `b` mutually depend on each other" (canonical, `a < b`).
    pub fn mutual(&mut self, a: AgentId, b: AgentId) -> i32 {
        self.id(VarKey::mutual(a, b))
    }

    /// Indicator "this concrete help event is asymmetric".
    pub fn asymmetric(
        &mut self,
        helper: AgentId,
        beneficiary: AgentId,
        pos: Position,
        t: usize,
    ) -> i32 {
        self.id(VarKey::asymmetric(helper, beneficiary, pos, t))
    }

    /// Indicator "`helper` has helped `beneficiary` at any time step ≤ `t`".
    pub fn has_helped_by_time(&mut self, helper: AgentId, beneficiary: AgentId, t: usize) -> i32 {
        self.id(VarKey::has_helped_by_time(helper, beneficiary, t))
    }

    /// Progress indicator: the first `step` edges of chain `chain_id` have fired by time `t`.
    pub fn chain_progress(&mut self, length: usize, chain_id: u32, step: u8, t: usize) -> i32 {
        self.id(VarKey::chain_progress(length, chain_id, step, t))
    }

    /// Whether chain `chain_id` has been fully realized.
    pub fn chain_realized(&mut self, length: usize, chain_id: u32) -> i32 {
        self.id(VarKey::chain_realized(length, chain_id))
    }

    /// Progress indicator: the first `step` edges of cycle rotation `cycle_id` have fired by time `t`.
    pub fn cycle_progress(&mut self, order: usize, cycle_id: u32, step: u8, t: usize) -> i32 {
        self.id(VarKey::cycle_progress(order, cycle_id, step, t))
    }

    /// Whether cycle rotation `cycle_id` has been fully realized.
    pub fn cycle_realized(&mut self, order: usize, cycle_id: u32) -> i32 {
        self.id(VarKey::cycle_realized(order, cycle_id))
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
