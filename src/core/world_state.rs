use crate::Position;
use std::hash::Hash;

#[derive(Debug, Clone)]
pub struct WorldState {
    pub agents_positions: Vec<Position>,
    pub gems_collected: Vec<bool>,
}

impl Hash for WorldState {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        for (i, j) in &self.agents_positions {
            i.hash(state);
            j.hash(state);
        }
        for gem in &self.gems_collected {
            gem.hash(state);
        }
    }
}
