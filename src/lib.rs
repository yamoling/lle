#[cfg(feature = "python-bindings")]
pub mod bindings;
pub mod continuous;
pub mod discrete;
pub mod shared;

pub use discrete::{
    Action, Agent, RuntimeWorldError, World, WorldState, rendering, rendering::Renderer, solver,
    tiles, tiles::Tile,
};
pub use shared::map::{MapSpec, ParseError, ValidMap, parse_toml as parse_v2};
pub use shared::{AgentId, BoxId, Grid, Position, WorldEvent};
// Include the version number of the crate from the build script
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(test)]
#[path = "unit_tests/test_architecture.rs"]
mod test_architecture;
