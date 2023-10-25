use crate::World;

pub enum DoneStrategy {
    /// The game ends when any agent dies or when all agents have arrived.
    Cooperarive,
    /// The game ends when an agent reaches the exit.
    Competitive,
}

impl DoneStrategy {
    pub fn is_done(&self, world: &World) -> bool {
        match self {
            DoneStrategy::Cooperarive => all_agents_arrived(world) || any_agent_dies(world),
            DoneStrategy::Competitive => any_alive_agent_arrived(world),
        }
    }
}

fn any_agent_dies(world: &World) -> bool {
    world.agents().iter().any(|agent| agent.is_dead())
}

fn all_agents_arrived(world: &World) -> bool {
    world.agents().iter().all(|agent| agent.has_arrived())
}

fn any_alive_agent_arrived(world: &World) -> bool {
    world
        .agents()
        .iter()
        .any(|agent| agent.has_arrived() && agent.is_alive())
}

#[cfg(test)]
#[path = "../unit_tests/test_done_strategy.rs"]
mod tests;
