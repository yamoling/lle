use crate::World;

use super::problem_state::ProblemState;

pub struct SearchProblem {
    world: World,
    initial_state: ProblemState,
    goal_state: ProblemState,
}

impl SearchProblem {
    pub fn new(world: World) -> Self {
        let initial_state = ProblemState::new(
            world.agent_positions().clone(),
            vec![false; world.n_gems() as usize],
        );
        let goal_state = ProblemState::new(
            world.agent_positions().clone(),
            vec![true; world.n_gems() as usize],
        );
        Self {
            world,
            initial_state,
            goal_state,
        }
    }

    pub fn initial_state(&self) -> &ProblemState {
        &self.initial_state
    }

    pub fn goal_state(&self) -> &ProblemState {
        &self.goal_state
    }

    pub fn world(&self) -> &World {
        &self.world
    }
}
