use crate::Position;
use std::hash::Hash;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldState {
    pub agents_positions: Vec<Position>,
    pub gems_collected: Vec<bool>,
    pub agents_alive: Vec<bool>,
}

impl WorldState {
    pub fn new_alive(agents_positions: Vec<Position>, gems_collected: Vec<bool>) -> Self {
        Self {
            agents_alive: vec![true; agents_positions.len()],
            agents_positions,
            gems_collected,
        }
    }
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
        for agent in &self.agents_alive {
            agent.hash(state);
        }
    }
}
