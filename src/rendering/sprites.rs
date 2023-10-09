use image::{self, RgbImage, RgbaImage};

use lazy_static::lazy_static;

use super::{BLACK, TILE_SIZE};

// Image binary data is included at compile time with the build.rs script.
// The environment variable OUT_DIR is set by Cargo
include!(concat!(env!("OUT_DIR"), "/constants.rs"));

fn load_rgba(bytes: &[&[u8]]) -> Vec<RgbaImage> {
    bytes
        .iter()
        .map(|bytes| {
            image::load_from_memory_with_format(bytes, image::ImageFormat::Png)
                .unwrap()
                .to_rgba8()
        })
        .collect()
}

fn load_rgb(bytes: &[&[u8]]) -> Vec<RgbImage> {
    bytes
        .iter()
        .map(|bytes| {
            image::load_from_memory_with_format(bytes, image::ImageFormat::Png)
                .unwrap()
                .to_rgb8()
        })
        .collect()
}

lazy_static! {
    pub static ref GEM: RgbaImage =
        image::load_from_memory_with_format(GEM_BYTES, image::ImageFormat::Png)
            .unwrap()
            .to_rgba8();
    pub static ref VOID: RgbaImage =
        image::load_from_memory_with_format(VOID_BYTES, image::ImageFormat::Png)
            .unwrap()
            .to_rgba8();
    pub static ref AGENTS: [RgbaImage; 4] = load_rgba(&AGENT_BYTES).try_into().unwrap();
    pub static ref HORIZONTAL_LASERS: [RgbaImage; 4] =
        load_rgba(&HORIZONTAL_LASER_BYTES).try_into().unwrap();
    pub static ref VERTICAL_LASERS: [RgbaImage; 4] =
        load_rgba(&VERTICAL_LASER_BYTES).try_into().unwrap();
    pub static ref LASER_SOURCES_NORTH: [RgbImage; 4] =
        load_rgb(&LASER_SOURCE_NORTH_BYTES).try_into().unwrap();
    pub static ref LASER_SOURCES_EAST: [RgbImage; 4] =
        load_rgb(&LASER_SOURCE_EAST_BYTES).try_into().unwrap();
    pub static ref LASER_SOURCES_SOUTH: [RgbImage; 4] =
        load_rgb(&LASER_SOURCE_SOUTH_BYTES).try_into().unwrap();
    pub static ref LASER_SOURCES_WEST: [RgbImage; 4] =
        load_rgb(&LASER_SOURCE_WEST_BYTES).try_into().unwrap();
    pub static ref WALL: RgbImage = RgbImage::from_pixel(TILE_SIZE, TILE_SIZE, BLACK);
}
