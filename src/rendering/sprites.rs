use image::{self, RgbImage, RgbaImage};

use super::{BLACK, TILE_SIZE};
use std::sync::LazyLock;

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

pub static GEM: LazyLock<RgbaImage> = LazyLock::new(|| {
    image::load_from_memory_with_format(GEM_BYTES, image::ImageFormat::Png)
        .unwrap()
        .to_rgba8()
});

pub static VOID: LazyLock<RgbaImage> = LazyLock::new(|| {
    image::load_from_memory_with_format(VOID_BYTES, image::ImageFormat::Png)
        .unwrap()
        .to_rgba8()
});

pub static AGENTS: LazyLock<[RgbaImage; 4]> =
    LazyLock::new(|| load_rgba(&AGENT_BYTES).try_into().unwrap());

pub static HORIZONTAL_LASERS: LazyLock<[RgbaImage; 4]> =
    LazyLock::new(|| load_rgba(&HORIZONTAL_LASER_BYTES).try_into().unwrap());

pub static VERTICAL_LASERS: LazyLock<[RgbaImage; 4]> =
    LazyLock::new(|| load_rgba(&VERTICAL_LASER_BYTES).try_into().unwrap());

pub static LASER_SOURCES_NORTH: LazyLock<[RgbImage; 4]> =
    LazyLock::new(|| load_rgb(&LASER_SOURCE_NORTH_BYTES).try_into().unwrap());

pub static LASER_SOURCES_EAST: LazyLock<[RgbImage; 4]> =
    LazyLock::new(|| load_rgb(&LASER_SOURCE_EAST_BYTES).try_into().unwrap());

pub static LASER_SOURCES_SOUTH: LazyLock<[RgbImage; 4]> =
    LazyLock::new(|| load_rgb(&LASER_SOURCE_SOUTH_BYTES).try_into().unwrap());

pub static LASER_SOURCES_WEST: LazyLock<[RgbImage; 4]> =
    LazyLock::new(|| load_rgb(&LASER_SOURCE_WEST_BYTES).try_into().unwrap());

pub static WALL: LazyLock<RgbImage> =
    LazyLock::new(|| RgbImage::from_pixel(TILE_SIZE, TILE_SIZE, BLACK));
