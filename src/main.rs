use env_logger;
use itertools::{iproduct, Itertools};
use std::collections::{HashMap, HashSet};
use std::f64::NEG_INFINITY;
use std::iter::repeat_with;

use lle::{World, WorldEvent, WorldState};

fn states(world: &World) -> impl Iterator<Item = WorldState> {
    let all_positions: HashSet<(usize, usize)> = (0..world.height())
        .flat_map(|i| (0..world.width()).map(move |j| (i, j)))
        .collect();
    let wall_pos = HashSet::from_iter(world.walls().into_iter());
    let valid_positions: Vec<(usize, usize)> =
        all_positions.difference(&wall_pos).cloned().collect();

    let agents_positions = repeat_with(|| valid_positions.clone())
        .take(world.n_agents())
        .multi_cartesian_product()
        // Remove states with agents in the same position
        .filter(|positions| {
            let mut set = HashSet::new();
            positions.iter().all(|&pos| set.insert(pos))
        });

    let gems_collection_status: Vec<_> = repeat_with(|| [true, false])
        .take(world.n_gems())
        .multi_cartesian_product()
        .collect();

    iproduct!(agents_positions, gems_collection_status).map(|(agents_positions, gems_collected)| {
        WorldState {
            agents_positions,
            gems_collected,
        }
    })
}

fn reward(events: Vec<WorldEvent>, mut n_arrived: usize, n_agents: usize) -> f64 {
    let mut r = 0.0;
    let mut r_death = 0.0;
    for event in events {
        match event {
            WorldEvent::GemCollected { .. } => r += 1.0,
            WorldEvent::AgentExit { .. } => {
                r += 1.0;
                n_arrived += 1;
            }
            WorldEvent::AgentDied { .. } => r_death -= 1.0,
        }
    }
    if r_death != 0.0 {
        return r_death;
    }
    if n_arrived == n_agents {
        r += 1.0;
    }
    r
}

fn is_final(world: &World) -> bool {
    for agent in world.agents() {
        if agent.is_dead() {
            log::debug!("Agent {agent:?} is dead");
            return true;
        }
    }
    for agent in world.agents() {
        if !agent.has_arrived() {
            log::debug!("Agent {agent:?} has not arrived");
            return false;
        }
    }
    true
}

fn value_iteration(mut world: World, n_iterations: usize) -> HashMap<WorldState, f64> {
    let mut value = HashMap::new();
    for i in 0..n_iterations {
        let start = std::time::Instant::now();
        log::info!("Started iteration {}", i + 1);
        let mut n_states = 0;
        let mut new_value = HashMap::new();
        for state in states(&world) {
            n_states += 1;
            log::debug!("{state:?}");
            world.set_state(&state).unwrap();
            if is_final(&world) {
                new_value.insert(state, 0.0);
                continue;
            }
            let mut max_value = NEG_INFINITY;
            for action in world.available_joint_actions() {
                world.set_state(&state).unwrap();
                let events = world.step(&action).unwrap();
                let next_state = world.get_state();
                let r = reward(events, world.n_agents_arrived(), world.n_agents());
                let value_action = r + value.get(&next_state).unwrap_or(&0.0);
                max_value = max_value.max(value_action);
            }
            new_value.insert(state, max_value);
        }
        let end = std::time::Instant::now();
        let duration = end - start;
        log::info!(
            "Iteration: {}, Number of states: {n_states}, duration: {duration:?}",
            i + 1
        );
        value = new_value;
    }
    value
}

fn main() {
    dotenv::dotenv().ok();
    env_logger::init();
    let w = World::get_level(6).unwrap();
    value_iteration(w, 100);
}
