//! Physics-agnostic building blocks shared by the discrete and the continuous engines: ids,
//! positions, grids, directions, events, sprites and the map layer.
//!
//! Nothing in this module may depend on the discrete or the continuous engine.
pub mod direction;
pub mod event;
pub mod grid;
pub mod ids;
pub mod map;
pub mod position;
pub(crate) mod sprites;
pub(crate) mod utils;

pub use direction::{CardinalDirection, Direction, VerticalDirection};
pub use event::WorldEvent;
pub use grid::Grid;
pub use ids::{AgentId, BoxId, Colour, LaserId};
pub use map::{MapSpec, ParseError, ValidMap};
pub use position::Position;
