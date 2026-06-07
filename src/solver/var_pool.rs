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

    pub fn id(&mut self, key: VarKey) -> i32 {
        if let Some(&id) = self.ids.get(&key) {
            return id;
        }
        // ids start at 1, as required by SAT solvers
        let id = (self.keys.len() + 1) as i32;
        self.keys.push(key);
        self.ids.insert(key, id);
        id
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
