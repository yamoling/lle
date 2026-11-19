mod renderer;
mod sprites;

pub use renderer::Renderer;

use image::Rgb;

pub const TILE_SIZE: u32 = 32;
const BACKGROUND_GREY: Rgb<u8> = Rgb([218, 218, 218]);
const GRID_GREY: Rgb<u8> = Rgb([127, 127, 127]);
const BLACK: Rgb<u8> = Rgb([0, 0, 0]);
/// Height in pixels of the band drawn between two floors of a multi-floor world.
pub const FLOOR_SEPARATOR_HEIGHT: u32 = TILE_SIZE / 4;
const FLOOR_SEPARATOR: Rgb<u8> = Rgb([40, 40, 40]);
