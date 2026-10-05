use image::{self, RgbImage, Rgba, RgbaImage};

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

fn load_single_rgba(bytes: &[u8]) -> RgbaImage {
    image::load_from_memory_with_format(bytes, image::ImageFormat::Png)
        .unwrap()
        .to_rgba8()
}

fn load_single_rgb(bytes: &[u8]) -> RgbImage {
    image::load_from_memory_with_format(bytes, image::ImageFormat::Png)
        .unwrap()
        .to_rgb8()
}

pub static AGENTS: LazyLock<Vec<RgbaImage>> = LazyLock::new(|| load_rgba(AGENT_BYTES));
pub static AGENT_FALLBACK: LazyLock<RgbaImage> =
    LazyLock::new(|| load_single_rgba(AGENT_FALLBACK_BYTES));

pub static HORIZONTAL_LASERS: LazyLock<Vec<RgbaImage>> =
    LazyLock::new(|| load_rgba(HORIZONTAL_LASER_BYTES));
pub static HORIZONTAL_LASER_FALLBACK: LazyLock<RgbaImage> =
    LazyLock::new(|| load_single_rgba(HORIZONTAL_LASER_FALLBACK_BYTES));

pub static VERTICAL_LASERS: LazyLock<Vec<RgbaImage>> =
    LazyLock::new(|| load_rgba(VERTICAL_LASER_BYTES));
pub static VERTICAL_LASER_FALLBACK: LazyLock<RgbaImage> =
    LazyLock::new(|| load_single_rgba(VERTICAL_LASER_FALLBACK_BYTES));

pub static LASER_SOURCES_NORTH: LazyLock<Vec<RgbImage>> =
    LazyLock::new(|| load_rgb(LASER_SOURCE_NORTH_BYTES));
pub static LASER_SOURCE_NORTH_FALLBACK: LazyLock<RgbImage> =
    LazyLock::new(|| load_single_rgb(LASER_SOURCE_NORTH_FALLBACK_BYTES));

pub static LASER_SOURCES_EAST: LazyLock<Vec<RgbImage>> =
    LazyLock::new(|| load_rgb(LASER_SOURCE_EAST_BYTES));
pub static LASER_SOURCE_EAST_FALLBACK: LazyLock<RgbImage> =
    LazyLock::new(|| load_single_rgb(LASER_SOURCE_EAST_FALLBACK_BYTES));

pub static LASER_SOURCES_SOUTH: LazyLock<Vec<RgbImage>> =
    LazyLock::new(|| load_rgb(LASER_SOURCE_SOUTH_BYTES));
pub static LASER_SOURCE_SOUTH_FALLBACK: LazyLock<RgbImage> =
    LazyLock::new(|| load_single_rgb(LASER_SOURCE_SOUTH_FALLBACK_BYTES));

pub static LASER_SOURCES_WEST: LazyLock<Vec<RgbImage>> =
    LazyLock::new(|| load_rgb(LASER_SOURCE_WEST_BYTES));
pub static LASER_SOURCE_WEST_FALLBACK: LazyLock<RgbImage> =
    LazyLock::new(|| load_single_rgb(LASER_SOURCE_WEST_FALLBACK_BYTES));

pub static WALL: LazyLock<RgbImage> =
    LazyLock::new(|| RgbImage::from_pixel(TILE_SIZE, TILE_SIZE, BLACK));

#[inline]
fn rgba_or_fallback<'a>(
    sprites: &'a [RgbaImage],
    fallback: &'a RgbaImage,
    agent_id: usize,
) -> &'a RgbaImage {
    sprites.get(agent_id).unwrap_or(fallback)
}

#[inline]
fn rgb_or_fallback<'a>(
    sprites: &'a [RgbImage],
    fallback: &'a RgbImage,
    agent_id: usize,
) -> &'a RgbImage {
    sprites.get(agent_id).unwrap_or(fallback)
}

#[inline]
pub fn agent(agent_id: usize) -> &'static RgbaImage {
    if agent_id <= MAX_NUMBERED_AGENT_SPRITE_ID {
        &AGENTS[agent_id]
    } else {
        &AGENT_FALLBACK
    }
}

#[inline]
pub fn horizontal_laser(agent_id: usize) -> &'static RgbaImage {
    rgba_or_fallback(&HORIZONTAL_LASERS, &HORIZONTAL_LASER_FALLBACK, agent_id)
}

#[inline]
pub fn vertical_laser(agent_id: usize) -> &'static RgbaImage {
    rgba_or_fallback(&VERTICAL_LASERS, &VERTICAL_LASER_FALLBACK, agent_id)
}

#[inline]
pub fn laser_source_north(agent_id: usize) -> &'static RgbImage {
    rgb_or_fallback(&LASER_SOURCES_NORTH, &LASER_SOURCE_NORTH_FALLBACK, agent_id)
}

#[inline]
pub fn laser_source_east(agent_id: usize) -> &'static RgbImage {
    rgb_or_fallback(&LASER_SOURCES_EAST, &LASER_SOURCE_EAST_FALLBACK, agent_id)
}

#[inline]
pub fn laser_source_south(agent_id: usize) -> &'static RgbImage {
    rgb_or_fallback(&LASER_SOURCES_SOUTH, &LASER_SOURCE_SOUTH_FALLBACK, agent_id)
}

#[inline]
pub fn laser_source_west(agent_id: usize) -> &'static RgbImage {
    rgb_or_fallback(&LASER_SOURCES_WEST, &LASER_SOURCE_WEST_FALLBACK, agent_id)
}

/// Drawn rather than loaded: a filled brown square with a darker border,
/// following `WALL`, which is also generated rather than shipped as a PNG.
pub static BOX: LazyLock<RgbaImage> = LazyLock::new(|| {
    const FILL: Rgba<u8> = Rgba([150, 103, 61, 255]);
    const BORDER: Rgba<u8> = Rgba([92, 60, 33, 255]);
    const MARGIN: u32 = 3;
    const THICKNESS: u32 = 2;
    let mut image = RgbaImage::from_pixel(TILE_SIZE, TILE_SIZE, Rgba([0, 0, 0, 0]));
    for x in MARGIN..TILE_SIZE - MARGIN {
        for y in MARGIN..TILE_SIZE - MARGIN {
            let on_border = x < MARGIN + THICKNESS
                || y < MARGIN + THICKNESS
                || x >= TILE_SIZE - MARGIN - THICKNESS
                || y >= TILE_SIZE - MARGIN - THICKNESS;
            image.put_pixel(x, y, if on_border { BORDER } else { FILL });
        }
    }
    image
});
