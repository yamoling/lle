use crate::Position;

pub struct ProblemState {
    agent_positions: Vec<Position>,
    gems_collected: Vec<bool>,
}

impl ProblemState {
    pub fn new(agent_positions: Vec<Position>, gems_collected: Vec<bool>) -> Self {
        Self {
            agent_positions,
            gems_collected,
        }
    }

    pub fn agent_positions(&self) -> &Vec<Position> {
        &self.agent_positions
    }

    pub fn gems_collected(&self) -> &Vec<bool> {
        &self.gems_collected
    }
}
