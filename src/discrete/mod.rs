//! The discrete (grid-based) engine: actions, agents, tiles, the world, the SAT solver and the
//! renderer.
mod action;
mod agent;
pub(crate) mod boxes;
mod errors;
mod position_ops;
pub mod rendering;
pub mod solver;
pub mod tiles;
mod world;
mod world_build;
mod world_state;

pub use action::Action;
pub use agent::Agent;
pub use boxes::BoxId;
pub use errors::RuntimeWorldError;
pub use world::World;
pub use world_state::WorldState;
