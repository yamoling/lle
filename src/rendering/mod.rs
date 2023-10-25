mod renderer;
mod sprites;
mod tile_visitor;

pub use renderer::{Renderer, VisitorData};
pub use tile_visitor::TileVisitor;

use image::Rgb;

pub const TILE_SIZE: u32 = 32;
const BACKGROUND_GREY: Rgb<u8> = Rgb([218, 218, 218]);
const GRID_GREY: Rgb<u8> = Rgb([127, 127, 127]);
const BLACK: Rgb<u8> = Rgb([0, 0, 0]);
