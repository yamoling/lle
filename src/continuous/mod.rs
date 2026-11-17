//! Continuous positions are (x, y): columns eastwards, rows southwards. A cell
//! (row, column) has centre (column + 0.5, row + 0.5), in world units.
//!
//! Ticks validate actions, compute intent, resolve collisions, process voids,
//! collect gems, enter exits, resolve laser deaths to a fixpoint, then advance time.
//! This ordering allows collecting a gem or arriving in the tick an agent dies.
//! Void contact is safe, but overlap kills: skirting a void requires radius clearance.
//! Between two voids, a one-cell corridor leaves 0.5 minus the radius on each side.
mod collision;
mod errors;
mod geometry;
mod lasers;
mod observations;
mod params;
mod state;
mod world;

pub use errors::ContinuousError;
pub use params::{ActionModel, PhysicsParams};
pub use state::ContinuousWorldState;
pub use world::{ContinuousWorld, LaserSourceInfo, RNG_STATE_SIZE, StepOutcome};
pub type ContinuousAction = [f64; 2];
