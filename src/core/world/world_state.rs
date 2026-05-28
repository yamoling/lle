use crate::Position;
use std::hash::Hash;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
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

    pub fn as_array(&self) -> Vec<f32> {
        let len = self.agents_positions.len() * 3 + self.gems_collected.len();
        let mut res = Vec::with_capacity(len);
        for pos in &self.agents_positions {
            res.push(pos.i as f32);
            res.push(pos.j as f32);
        }
        for is_collected in &self.gems_collected {
            res.push(if *is_collected { 1.0 } else { 0.0 });
        }
        for alive in &self.agents_alive {
            res.push(if *alive { 1.0 } else { 0.0 });
        }
        res
    }
}
