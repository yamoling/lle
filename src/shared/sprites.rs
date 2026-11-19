use image::{self, Rgb, RgbImage, RgbaImage};

use std::sync::LazyLock;

pub const TILE_SIZE: u32 = 32;
pub const BACKGROUND_GREY: Rgb<u8> = Rgb([218, 218, 218]);
pub const GRID_GREY: Rgb<u8> = Rgb([127, 127, 127]);
pub const BLACK: Rgb<u8> = Rgb([0, 0, 0]);
/// Height in pixels of the band drawn between two floors of a multi-floor world.
pub const FLOOR_SEPARATOR_HEIGHT: u32 = TILE_SIZE / 4;
pub const FLOOR_SEPARATOR: Rgb<u8> = Rgb([40, 40, 40]);

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

pub static BOX: LazyLock<RgbaImage> = LazyLock::new(|| {
    image::load_from_memory_with_format(BOX_BYTES, image::ImageFormat::Png)
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

pub static LIFT_UP: LazyLock<RgbaImage> = LazyLock::new(|| {
    image::load_from_memory_with_format(LIFT_UP_BYTES, image::ImageFormat::Png)
        .unwrap()
        .to_rgba8()
});

pub static LIFT_DOWN: LazyLock<RgbaImage> = LazyLock::new(|| {
    image::load_from_memory_with_format(LIFT_DOWN_BYTES, image::ImageFormat::Png)
        .unwrap()
        .to_rgba8()
});

/// The lift sprites drawn while the lift's group is powered (`World::powered_groups`).
pub static LIFT_UP_ACTIVE: LazyLock<RgbaImage> =
    LazyLock::new(|| load_single_rgba(LIFT_UP_ACTIVE_BYTES));

pub static LIFT_DOWN_ACTIVE: LazyLock<RgbaImage> =
    LazyLock::new(|| load_single_rgba(LIFT_DOWN_ACTIVE_BYTES));

pub static BUTTON_IDLE: LazyLock<RgbaImage> = LazyLock::new(|| {
    image::load_from_memory_with_format(BUTTON_IDLE_BYTES, image::ImageFormat::Png)
        .unwrap()
        .to_rgba8()
});

pub static BUTTON_PRESSED: LazyLock<RgbaImage> = LazyLock::new(|| {
    image::load_from_memory_with_format(BUTTON_PRESSED_BYTES, image::ImageFormat::Png)
        .unwrap()
        .to_rgba8()
});

pub static LEVER_OFF: LazyLock<RgbaImage> = LazyLock::new(|| load_single_rgba(LEVER_OFF_BYTES));

pub static LEVER_ON: LazyLock<RgbaImage> = LazyLock::new(|| load_single_rgba(LEVER_ON_BYTES));

pub static PRESSURE_PLATE_IDLE: LazyLock<RgbaImage> =
    LazyLock::new(|| load_single_rgba(PRESSURE_PLATE_IDLE_BYTES));

pub static PRESSURE_PLATE_PRESSED: LazyLock<RgbaImage> =
    LazyLock::new(|| load_single_rgba(PRESSURE_PLATE_PRESSED_BYTES));

pub static AGENT_LOCK: LazyLock<RgbaImage> = LazyLock::new(|| {
    image::load_from_memory_with_format(AGENT_LOCK_BYTES, image::ImageFormat::Png)
        .unwrap()
        .to_rgba8()
});

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

/// The flat accent colour of each numbered agent sprite (`agents/<c>.png`), indexed by
/// `Colour`: red, yellow, green, blue, magenta, orange, purple, cyan, brown, lime and teal.
///
/// It tints the lift/button restriction badges of the grid renderer and draws the agents and
/// beams of the continuous renderer. Sprite 11 is neutral and has no accent, so it falls
/// through to [`AGENT_COLOUR_FALLBACK`] along with every colour past the numbered sprites: a
/// colour has no upper bound, so index this through [`agent_colour`] and never directly.
pub const AGENT_COLOURS: [Rgb<u8>; 11] = [
    Rgb([203, 52, 52]),  // 0  red
    Rgb([203, 178, 52]), // 1  yellow
    Rgb([52, 203, 77]),  // 2  green
    Rgb([51, 38, 190]),  // 3  blue
    Rgb([203, 52, 148]), // 4  magenta
    Rgb([203, 122, 52]), // 5  orange
    Rgb([140, 52, 203]), // 6  purple
    Rgb([52, 190, 203]), // 7  cyan
    Rgb([126, 73, 32]),  // 8  brown
    Rgb([148, 203, 52]), // 9  lime
    Rgb([52, 203, 173]), // 10 teal
];

/// The tint of a colour past the numbered sprites. Those colours all share one fallback agent
/// sprite, so they share one fallback tint.
pub const AGENT_COLOUR_FALLBACK: Rgb<u8> = Rgb([150, 150, 150]);

/// The accent colour of `colour`, falling back instead of panicking when the colour runs past
/// the table, with the same contract as [`agent`].
pub fn agent_colour(colour: usize) -> Rgb<u8> {
    *AGENT_COLOURS.get(colour).unwrap_or(&AGENT_COLOUR_FALLBACK)
}
