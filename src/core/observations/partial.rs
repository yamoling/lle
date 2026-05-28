use super::ObservationGenerator;
use crate::{Position, World};
use ndarray::{Array4, ArrayD, Axis};

#[derive(Clone)]
pub struct Partial {
    size: usize,
    center: i32,
    n_agents: usize,
    wall_idx: usize,
    laser_0_idx: usize,
    gem_idx: usize,
    exit_idx: usize,
    n_layers: usize,
}

impl Partial {
    pub fn new(world: &World, size: usize) -> Self {
        let n_agents = world.n_agents();
        let wall_idx = n_agents;
        let laser_0_idx = wall_idx + 1;
        let gem_idx = laser_0_idx + n_agents;
        let exit_idx = gem_idx + 1;
        let n_layers = exit_idx + 1;

        Self {
            size,
            center: (size / 2) as i32,
            n_agents,
            wall_idx,
            laser_0_idx,
            gem_idx,
            exit_idx,
            n_layers,
        }
    }

    fn encode_layer(
        &self,
        mut layer: ndarray::ArrayViewMut2<f32>,
        origin: &Position,
        positions: &[Position],
        fill_value: f32,
    ) {
        for pos in positions {
            let i = pos.i as i32 - origin.i as i32 + self.center;
            let j = pos.j as i32 - origin.j as i32 + self.center;
            if i >= 0 && i < self.size as i32 && j >= 0 && j < self.size as i32 {
                layer[[i as usize, j as usize]] = fill_value;
            }
        }
    }
}

impl ObservationGenerator for Partial {
    fn observe(&self, world: &World) -> ArrayD<f32> {
        let mut obs = Array4::zeros((self.n_agents, self.n_layers, self.size, self.size));
        let agent_positions = world.agents_positions();

        for (a, agent_pos) in agent_positions.iter().enumerate() {
            let mut agent_obs = obs.index_axis_mut(Axis(0), a);

            // Other agents
            for (a2, other_pos) in agent_positions.iter().enumerate() {
                self.encode_layer(
                    agent_obs.index_axis_mut(Axis(0), a2),
                    agent_pos,
                    &[*other_pos],
                    1.0,
                );
            }

            // Gems
            let gems: Vec<Position> = world
                .gems()
                .iter()
                .filter(|(_, g)| !g.is_collected())
                .map(|(p, _)| *p)
                .collect();
            self.encode_layer(
                agent_obs.index_axis_mut(Axis(0), self.gem_idx),
                agent_pos,
                &gems,
                1.0,
            );

            // Exits
            self.encode_layer(
                agent_obs.index_axis_mut(Axis(0), self.exit_idx),
                agent_pos,
                &world.exits_positions(),
                1.0,
            );

            // Walls
            self.encode_layer(
                agent_obs.index_axis_mut(Axis(0), self.wall_idx),
                agent_pos,
                &world.walls(),
                1.0,
            );

            // Lasers
            for (pos, laser) in world.lasers() {
                if laser.is_on() {
                    self.encode_layer(
                        agent_obs.index_axis_mut(Axis(0), self.laser_0_idx + laser.agent_id()),
                        agent_pos,
                        &[pos],
                        1.0,
                    );
                }
            }

            // Laser sources
            for (pos, source) in world.sources() {
                self.encode_layer(
                    agent_obs.index_axis_mut(Axis(0), self.laser_0_idx + source.agent_id()),
                    agent_pos,
                    &[pos],
                    -1.0,
                );
            }
        }

        obs.into_dyn()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::World;

    #[test]
    fn test_partial_shape() {
        let world = World::try_from("S0 . X").unwrap();
        let partial_gen = Partial::new(&world, 3);
        let obs = partial_gen.observe(&world);
        // (n_agents, n_layers, size, size)
        // 1 agent, layers: A0, Wall, Laser_0 (none), Void, Gem (none), Exit
        // So 5 layers if I logic correctly?
        // Wait, Partial always has n_agents + n_agents + 3 layers.
        // 1 + 1 + 3 = 5.
        assert_eq!(obs.shape(), &[1, 5, 3, 3]);
    }

    #[test]
    fn test_partial_content() {
        let world = World::try_from("S0 @ X").unwrap();
        let partial_gen = Partial::new(&world, 3);
        let obs = partial_gen.observe(&world);

        // Agent 0 is at (0, 0), which is center of its 3x3 view (if it was padded?)
        // Wait, encode_layer uses center = size / 2.
        // center = 1.
        // pos (0, 0) relative to agent at (0, 0) is (1, 1).
        assert_eq!(obs[[0, 0, 1, 1]], 1.0); // A0
        // Wall at (0, 1) relative to agent at (0, 0) is (1, 2).
        assert_eq!(obs[[0, 1, 1, 2]], 1.0); // Wall
        // Exit at (0, 2) relative to agent at (0, 0) is (1, 3) -> OUT OF BOUNDS
    }
}
