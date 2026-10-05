use crate::Position;
use std::hash::Hash;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct WorldState {
    pub agents_positions: Vec<Position>,
    pub gems_collected: Vec<bool>,
    pub agents_alive: Vec<bool>,
    pub boxes_positions: Vec<Position>,
    pub boxes_present: Vec<bool>,
}

impl WorldState {
    pub fn new_alive(agents_positions: Vec<Position>, gems_collected: Vec<bool>) -> Self {
        Self {
            agents_alive: vec![true; agents_positions.len()],
            agents_positions,
            gems_collected,
            boxes_positions: vec![],
            boxes_present: vec![],
        }
    }
}
