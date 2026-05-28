use crate::World;
use ndarray::ArrayD;

mod layered;
mod partial;
mod state;
pub use layered::Layered;
pub use partial::Partial;
pub use state::State;

pub trait ObservationGenerator {
    fn observe(&self, world: &World) -> ArrayD<f32>;
}
