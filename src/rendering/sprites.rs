use image::{self, RgbImage, Rgba, RgbaImage};

use lazy_static::lazy_static;

use super::{AGENT_COLOURS, BACKGROUND_GREY, BLACK, TILE_SIZE};

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
    pub static ref GEM: RgbaImage = image::load_from_memory_with_format(
        include_bytes!("../../resources/gem.png"),
        image::ImageFormat::Png
    )
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
    pub static ref FLOOR: RgbImage = RgbImage::from_pixel(TILE_SIZE, TILE_SIZE, BACKGROUND_GREY);
    pub static ref FINISH: RgbImage = RgbImage::from_fn(TILE_SIZE, TILE_SIZE, |x, y| {
        const THICKNESS: u32 = 3u32;
        if x < THICKNESS || y < THICKNESS {
            return BLACK;
        }
        if x > TILE_SIZE - THICKNESS || y > TILE_SIZE - THICKNESS {
            return BLACK;
        }
        BACKGROUND_GREY
    });
    pub static ref STARTS: [RgbaImage; 4] = AGENT_COLOURS
        .iter()
        .map(|colour| {
            RgbaImage::from_fn(TILE_SIZE, TILE_SIZE, |x, y| {
                const THICKNESS: u32 = 3u32;
                if x < THICKNESS || y < THICKNESS {
                    return Rgba([colour.0[0], colour.0[1], colour.0[2], 255u8]);
                }
                if x > TILE_SIZE - THICKNESS || y > TILE_SIZE - THICKNESS {
                    return Rgba([colour.0[0], colour.0[1], colour.0[2], 255u8]);
                }
                Rgba([0, 0, 0, 0])
            })
        })
        .collect::<Vec<RgbaImage>>()
        .try_into()
        .unwrap();
    pub static ref WALL: RgbImage = RgbImage::from_pixel(TILE_SIZE, TILE_SIZE, BLACK);
}
