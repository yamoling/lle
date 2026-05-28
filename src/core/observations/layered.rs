use super::ObservationGenerator;
use crate::{Position, World};
use ndarray::{Array3, Array4, ArrayD, Axis};

#[derive(Clone)]
pub struct Layered {
    static_obs: Array3<f32>,
    n_agents: usize,
    width: usize,
    height: usize,
    a0_idx: usize,
    wall_idx: usize,
    laser_0_idx: usize,
    void_idx: usize,
    gem_idx: usize,
    exit_idx: usize,
    shape: (usize, usize, usize),
}

impl Layered {
    pub fn new(world: &World) -> Self {
        Self::new_padded(world, world.n_agents())
    }

    pub fn new_padded(world: &World, n_agents: usize) -> Self {
        let width = world.width();
        let height = world.height();
        let highest_laser_agent_id = world
            .sources()
            .iter()
            .map(|(_, s)| s.agent_id())
            .max()
            .unwrap_or(0);
        let n_laser_layers = highest_laser_agent_id + 1;

        let a0_idx = 0;
        let wall_idx = a0_idx + n_agents;
        let laser_0_idx = wall_idx + 1;
        let void_idx = laser_0_idx + n_laser_layers;
        let gem_idx = void_idx + 1;
        let exit_idx = gem_idx + 1;
        let n_layers = exit_idx + 1;
        let shape = (n_layers, height, width);

        let mut static_obs = Array3::zeros(shape);
        for pos in world.walls() {
            static_obs[[wall_idx, pos.i, pos.j]] = 1.0;
        }
        for pos in world.void_positions() {
            static_obs[[void_idx, pos.i, pos.j]] = 1.0;
        }

        Self {
            static_obs,
            n_agents,
            width,
            height,
            a0_idx,
            wall_idx,
            laser_0_idx,
            void_idx,
            gem_idx,
            exit_idx,
            shape,
        }
    }

    pub fn shape(&self) -> (usize, usize, usize) {
        self.shape
    }
}

impl ObservationGenerator for Layered {
    fn observe(&self, world: &World) -> ArrayD<f32> {
        let mut obs = self.static_obs.clone();

        for pos in world.exits_positions() {
            obs[[self.exit_idx, pos.i, pos.j]] = 1.0;
        }
        for (pos, source) in world.sources() {
            obs[[self.laser_0_idx + source.agent_id(), pos.i, pos.j]] = -1.0;
        }
        for (pos, laser) in world.lasers() {
            if laser.is_on() {
                obs[[self.laser_0_idx + laser.agent_id(), pos.i, pos.j]] = 1.0;
            }
        }
        for (pos, gem) in world.gems() {
            if !gem.is_collected() {
                obs[[self.gem_idx, pos.i, pos.j]] = 1.0;
            }
        }
        for (i, pos) in world.agents_positions().iter().enumerate() {
            obs[[self.a0_idx + i, pos.i, pos.j]] = 1.0;
        }

        // Tile the observation for each agent
        let mut tiled_obs =
            Array4::zeros((self.n_agents, self.shape.0, self.shape.1, self.shape.2));
        for mut agent_obs in tiled_obs.axis_iter_mut(Axis(0)) {
            agent_obs.assign(&obs);
        }
        tiled_obs.into_dyn()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::World;

    #[test]
    fn test_layered_shape() {
        let world = World::try_from("S0 . X").unwrap();
        let layered = Layered::new(&world);
        let obs = layered.observe(&world);
        assert_eq!(obs.shape(), &[1, 6, 1, 3]); // 1 agent, 6 layers (A0, Wall, Laser_0, Void, Gem, Exit), 1x3 grid
    }

    #[test]
    fn test_layered_content() {
        let world = World::try_from("S0 @ G X").unwrap();
        // Layers: A0, Wall, Laser_0, Void, Gem, Exit
        let layered = Layered::new(&world);
        let obs = layered.observe(&world);
        assert_eq!(obs.shape(), &[1, 6, 1, 4]);

        // Agent 0 at (0, 0)
        assert_eq!(obs[[0, 0, 0, 0]], 1.0);
        // Wall at (0, 1)
        assert_eq!(obs[[0, 1, 0, 1]], 1.0);
        // Void at (no void)
        // Gem at (0, 2)
        assert_eq!(obs[[0, 4, 0, 2]], 1.0); // A0=0, Wall=1, Laser=2, Void=3, Gem=4
        // Exit at (0, 3)
        assert_eq!(obs[[0, 5, 0, 3]], 1.0); // Exit=5
    }
}
