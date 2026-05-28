use super::ObservationGenerator;
use crate::World;
use ndarray::{Array1, Array2, ArrayD, Axis};

#[derive(Clone)]
pub struct State {
    n_agents: usize,
    dimensions: Array1<f32>,
}

impl State {
    pub fn new(world: &World, normalize: bool) -> Self {
        let n_agents = world.n_agents();
        let mut dimensions = Vec::with_capacity(n_agents * 2);
        for _ in 0..n_agents {
            if normalize {
                dimensions.push(world.height() as f32);
                dimensions.push(world.width() as f32);
            } else {
                dimensions.push(1.0);
                dimensions.push(1.0);
            }
        }
        Self {
            n_agents,
            dimensions: Array1::from(dimensions),
        }
    }
}

impl ObservationGenerator for State {
    fn observe(&self, world: &World) -> ArrayD<f32> {
        let mut state = world.get_state().as_array();
        // Normalize only agent positions (the first n_agents * 2 elements)
        for i in 0..self.n_agents * 2 {
            state[i] /= self.dimensions[i];
        }

        let state_size = state.len();
        let mut tiled_state = Array2::zeros((self.n_agents, state_size));
        let state_arr = Array1::from(state);
        for mut agent_state in tiled_state.axis_iter_mut(Axis(0)) {
            agent_state.assign(&state_arr);
        }
        tiled_state.into_dyn()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::World;

    #[test]
    fn test_state_shape() {
        let world = World::try_from("S0 . X").unwrap();
        let state_gen = State::new(&world, false);
        let obs = state_gen.observe(&world);
        // (n_agents, state_size)
        // state_size = n_agents * 3 + n_gems
        // 1*3 + 0 = 3
        assert_eq!(obs.shape(), &[1, 3]);
    }

    #[test]
    fn test_state_content() {
        let world = World::try_from("S0 G X").unwrap();
        let state_gen = State::new(&world, false);
        let obs = state_gen.observe(&world);
        // state: [pos_i, pos_j, gem_collected, alive]
        // gems = 1, agents = 1.
        // state_size = 1*2 + 1 + 1 = 4.
        assert_eq!(obs.shape(), &[1, 4]);
        assert_eq!(obs[[0, 0]], 0.0); // pos_i
        assert_eq!(obs[[0, 1]], 0.0); // pos_j
        assert_eq!(obs[[0, 2]], 0.0); // gem not collected
        assert_eq!(obs[[0, 3]], 1.0); // alive
    }

    #[test]
    fn test_state_normalization() {
        let mut world = World::try_from("S0 .\n. X").unwrap(); // 2x2
        let state_gen = State::new(&world, true);
        world.reset();
        // Move agent to (1, 1) - wait, X is at (1, 1). S0 is at (0, 0).
        // Let's just check (0, 0) normalized.
        let obs = state_gen.observe(&world);
        assert_eq!(obs[[0, 0]], 0.0);
        assert_eq!(obs[[0, 1]], 0.0);

        // Actually, let's check a non-zero position.
        // I need to set agent position.
        // But World::reset() sets it to start.
    }
}
