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
    /// (laser_id, i, j, t)
    Laser {
        laser_id: AgentId,
        pos: Position,
        t: usize,
    },
    /// Whether `helper` has helped `beneficiary` at any time step ≤ `t` (a monotone temporal
    /// prefix-OR over the per-step help events). This is the single shared "h has helped b"
    /// indicator: read at the current horizon it expresses time-agnostic dependency (used by
    /// the mutual-cooperation forbid), and it seeds the first edge of every temporal walk
    /// (chains and interdependence cycles).
    FirstHelpedByTime {
        helper: AgentId,
        beneficiary: AgentId,
        t: usize,
    },
    /// Whether agents `a` and `b` mutually depend on each other (canonical: `a < b`).
    Mutual { a: AgentId, b: AgentId },
    /// Progress for temporal walk `walk_id`: its first `step` edges have fired with
    /// non-decreasing timestamps, the `step`-th edge firing at some time ≤ `t`. Only created for
    /// `step ≥ 2`; the first edge is expressed directly by [`FirstHelpedByTime`]. A walk is a
    /// chain (`a → b → c`, open) or an interdependence cycle (closed) depending on the mode.
    WalkProgress { walk_id: u32, step: u8, t: usize },
    /// Whether temporal walk `walk_id` has been fully realized (all its edges fired in order).
    /// Subsumes the former `Chain` (open length-2 walk) and `CycleRealized` (closed walk).
    WalkRealized { walk_id: u32 },
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
    pub fn first_helped_by_time(helper: AgentId, beneficiary: AgentId, t: usize) -> Self {
        VarKey::FirstHelpedByTime {
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
    pub fn walk_progress(walk_id: u32, step: u8, t: usize) -> Self {
        VarKey::WalkProgress { walk_id, step, t }
    }

    #[inline]
    pub fn walk_realized(walk_id: u32) -> Self {
        VarKey::WalkRealized { walk_id }
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

    /// Indicator "`helper` has helped `beneficiary` at any time step ≤ `t`".
    pub fn first_helped_by_time(&mut self, helper: AgentId, beneficiary: AgentId, t: usize) -> i32 {
        self.id(VarKey::first_helped_by_time(helper, beneficiary, t))
    }

    /// Progress indicator: the first `step` edges of walk `walk_id` have been fired by time `t`.
    pub fn walk_progress(&mut self, walk_id: u32, step: u8, t: usize) -> i32 {
        self.id(VarKey::walk_progress(walk_id, step, t))
    }

    /// Whether temporal walk `walk_id` has been fully realized.
    pub fn walk_realized(&mut self, walk_id: u32) -> i32 {
        self.id(VarKey::walk_realized(walk_id))
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

        let mut plan = Vec::with_capacity(t_end);
        for t in 0..t_end {
            let mut row = Vec::with_capacity(agent_ids.len());
            for &agent in &agent_ids {
                let (prev, current) = (positions[&agent][&t], positions[&agent][&(t + 1)]);
                let Position { i: y1, j: x1 } = prev;
                let Position { i: y2, j: x2 } = current;
                let (dx, dy) = (x2 as i64 - x1 as i64, y2 as i64 - y1 as i64);
                let action = match (dx, dy) {
                    (0, 0) => Action::Stay,
                    (0, -1) => Action::North,
                    (0, 1) => Action::South,
                    (1, 0) => Action::East,
                    (-1, 0) => Action::West,
                    _ => {
                        return Err(SolverError::InvalidTrajectory {
                            prev_pos: prev,
                            current_pos: current,
                            agent,
                            index: t + 1,
                        });
                    }
                };
                row.push(action);
            }
            plan.push(row);
        }
        Ok(plan)
    }
}
