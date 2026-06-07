use std::collections::HashMap;

/// Semantic key for a SAT variable, mirroring the Python `VariableFactory` pool.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum VarKey {
    /// (agent, i, j, t)
    Agent(usize, usize, usize, usize),
    /// (laser_id, i, j, t)
    Laser(usize, usize, usize, usize),
    /// Auxiliary variable used internally by cardinality encodings; carries a unique counter.
    Aux(i32),
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

    pub fn agent(&mut self, agent: usize, i: usize, j: usize, t: usize) -> i32 {
        self.id(VarKey::Agent(agent, i, j, t))
    }

    pub fn laser(&mut self, laser: usize, i: usize, j: usize, t: usize) -> i32 {
        self.id(VarKey::Laser(laser, i, j, t))
    }

    fn next_id(&self) -> i32 {
        // ids start at 1, as required by SAT solvers
        self.ids.len() as i32 + 1
    }

    pub fn aux(&mut self) -> i32 {
        self.id(VarKey::Aux(self.next_id()))
    }

    pub fn get(&self, key: &VarKey) -> Option<i32> {
        self.ids.get(key).copied()
    }

    pub fn key(&self, id: i32) -> Option<VarKey> {
        if id <= 0 {
            return None;
        }
        self.keys.get((id - 1) as usize).copied()
    }
}
