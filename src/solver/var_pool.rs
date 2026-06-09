use std::collections::HashMap;

use crate::{Action, AgentId, Position};

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
    /// Whether laser `laser_id`'s beam is blocked by its same-colour agent at time `t`.
    /// This variable is used to identify cooperation, not to enforce domain consistency.
    LaserBlocked { laser_id: usize, t: usize },
    /// Whether `helper` blocks laser `laser_id` while `beneficiary` occupies a protected
    /// downstream beam position at time `t` (a single concrete cooperation event).
    CoopEvent {
        helper: AgentId,
        beneficiary: AgentId,
        laser_id: usize,
        t: usize,
    },
    /// Whether `helper` ever helps `beneficiary` at some point across the whole horizon.
    DependsOn {
        beneficiary: AgentId,
        helper: AgentId,
    },
    /// Whether `helper` has helped `beneficiary` at any time step ≤ `t` (running OR over
    /// `coop_event` at steps 0..=t). Used as the "left side" of a temporal chain.
    FirstHelpedByTime {
        helper: AgentId,
        beneficiary: AgentId,
        t: usize,
    },
    /// Whether a temporal chain `a → b → c` exists: `a` helped `b` at time `t-1` or earlier
    /// and `b` helped `c` at exactly time `t`. Auxiliary; only `Chain` is exposed externally.
    ChainEvent {
        a: AgentId,
        b: AgentId,
        c: AgentId,
        t: usize,
    },
    /// Whether a temporal chain `a → b → c` exists across the whole horizon: `a` helped `b`
    /// at some time strictly before `b` helped `c`.
    Chain { a: AgentId, b: AgentId, c: AgentId },
    /// Whether agents `a` and `b` mutually depend on each other (canonical: `a < b`).
    Mutual { a: AgentId, b: AgentId },
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
    pub fn laser_blocked(laser_id: usize, t: usize) -> Self {
        VarKey::LaserBlocked { laser_id, t }
    }

    #[inline]
    pub fn coop_event(helper: AgentId, beneficiary: AgentId, laser_id: usize, t: usize) -> Self {
        VarKey::CoopEvent {
            helper,
            beneficiary,
            laser_id,
            t,
        }
    }

    #[inline]
    pub fn depends_on(beneficiary: AgentId, helper: AgentId) -> Self {
        VarKey::DependsOn {
            beneficiary,
            helper,
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

    #[inline]
    pub fn chain_event(a: AgentId, b: AgentId, c: AgentId, t: usize) -> Self {
        VarKey::ChainEvent { a, b, c, t }
    }

    #[inline]
    pub fn chain(a: AgentId, b: AgentId, c: AgentId) -> Self {
        VarKey::Chain { a, b, c }
    }

    /// Canonical (min < max) mutual-dependency key for the unordered pair `{a, b}`.
    #[inline]
    pub fn mutual(a: AgentId, b: AgentId) -> Self {
        let (lo, hi) = if a < b { (a, b) } else { (b, a) };
        VarKey::Mutual { a: lo, b: hi }
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

    pub fn laser_blocked(&mut self, laser_id: usize, t: usize) -> i32 {
        self.id(VarKey::LaserBlocked { laser_id, t })
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

    /// Decode a SAT model (list of signed literals) into a joint action plan of length `t_end`.
    pub fn decode_plan(&self, literals: &[i32], t_end: usize) -> Result<Vec<Vec<Action>>, String> {
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
                let Position { i: y1, j: x1 } = positions[&agent][&t];
                let Position { i: y2, j: x2 } = positions[&agent][&(t + 1)];
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
                        ));
                    }
                };
                row.push(action);
            }
            plan.push(row);
        }
        Ok(plan)
    }
}
