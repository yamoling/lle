use crate::World;
use numpy::ndarray::{Array2, Array3, Array4};

pub fn observe_relative_positions(world: &World) -> Vec<Vec<f32>> {
    let height = world.height() as f32;
    let width = world.width() as f32;
    let state = world.get_state();
    let positions = state
        .agents_positions
        .iter()
        .map(|&(y, x)| (y as f32 / height, x as f32 / width))
        .flat_map(|(y, x)| vec![y, x])
        .collect::<Vec<f32>>();
    let gems = state
        .gems_collected
        .iter()
        .map(|&n| if n { 1f32 } else { 0f32 })
        .collect::<Vec<f32>>();
    let res = vec![positions, gems].concat();
    vec![res; world.n_agents()]
}

pub struct Layered {
    width: usize,
    height: usize,
    n_agents: usize,
    layered_shape: (usize, usize, usize, usize),
    flat_shape: (usize, usize),
    a0_idx: usize,
    wall: usize,
    laser_0_idx: usize,
    gem_idx: usize,
    exit: usize,
    static_obs: Array3<f32>,
}

impl Layered {
    pub fn new(world: &World) -> Self {
        let width = world.width();
        let height = world.height();
        let n_agents = world.n_agents();
        let shape = (n_agents * 2 + 3, height, width);
        let a0 = 0;
        let wall = a0 + n_agents;
        let laser_0 = wall + 1;
        let gem = laser_0 + n_agents;
        let exit = gem + 1;
        let static_obs = Layered::setup_static_obs(world, shape, exit, wall, laser_0);
        let layered_shape = (n_agents, shape.0, shape.1, shape.2);
        let flat_shape = (n_agents, shape.0 * shape.1 * shape.2);

        Layered {
            width,
            height,
            n_agents,
            layered_shape,
            flat_shape,
            a0_idx: a0,
            wall,
            laser_0_idx: laser_0,
            gem_idx: gem,
            exit,
            static_obs,
        }
    }

    fn setup_static_obs(
        world: &World,
        shape: (usize, usize, usize),
        exit: usize,
        wall: usize,
        laser_0: usize,
    ) -> Array3<f32> {
        let mut obs = Array3::zeros(shape);

        for &(i, j) in world.exits() {
            obs[[exit, i, j]] = 1f32;
        }

        for &(i, j) in world.walls() {
            obs[[wall, i, j]] = 1f32;
        }

        for (&(i, j), source) in world.laser_sources() {
            obs[[laser_0 + source.agent_id(), i, j]] = -1f32;
        }
        obs
    }

    pub fn observe(&self, world: &World) -> Array4<f32> {
        let mut obs = self.static_obs.clone();

        for (&(i, j), laser) in world.lasers() {
            if laser.is_on() {
                obs[[self.laser_0_idx + laser.agent_id(), i, j]] = 1f32;
            }
        }

        for (&(i, j), gem) in world.gems() {
            if !gem.is_collected() {
                obs[[self.gem_idx, i, j]] = 1f32;
            }
        }

        for (a, &(i, j)) in world.agents_positions().iter().enumerate() {
            obs[[self.a0_idx + a, i, j]] = 1f32;
        }

        // Tile the array to match the number of agents
        let as_vec = vec![obs.into_raw_vec(); world.n_agents()].concat();
        Array4::from_shape_vec(self.layered_shape, as_vec).unwrap()
    }

    pub fn observe_flat(&self, world: &World) -> Array2<f32> {
        let obs = self.observe(world);
        obs.into_shape(self.flat_shape).unwrap()
    }
}
