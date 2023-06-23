use image::{self, RgbImage, Rgba, RgbaImage};

use lazy_static::lazy_static;

use super::{AGENT_COLOURS, BACKGROUND_GREY, BLACK, STR_AGENT_COLOURS, TILE_SIZE};

lazy_static! {
    pub static ref GEM: RgbaImage = image::open("src/oxen/rendering/sprites/gem.png")
        .unwrap()

        .to_rgba8();
    pub static ref AGENTS: [RgbaImage; 4] = STR_AGENT_COLOURS
        .iter()
        .map(|colour| {
            image::open(format!("src/oxen/rendering/sprites/agents/{colour}.png"))
                .unwrap()
                .to_rgba8()
        })
        .collect::<Vec<RgbaImage>>()
        .try_into()
        .unwrap();
    pub static ref HORIZONTAL_LASERS: [RgbaImage; 4] = STR_AGENT_COLOURS
        .iter()
        .map(|colour| {
            image::open(format!(
                "src/oxen/rendering/sprites/lasers/horizontal/{colour}.png"
            ))
            .unwrap()
            .to_rgba8()
        })
        .collect::<Vec<RgbaImage>>()
        .try_into()
        .unwrap();
    pub static ref VERTICAL_LASERS: [RgbaImage; 4] = STR_AGENT_COLOURS
        .iter()
        .map(|colour| {
            image::open(format!(
                "src/oxen/rendering/sprites/lasers/vertical/{colour}.png"
            ))
            .unwrap()
            .to_rgba8()
        })
        .collect::<Vec<RgbaImage>>()
        .try_into()
        .unwrap();
    pub static ref LASER_SOURCES_NORTH: [RgbImage; 4] = STR_AGENT_COLOURS
        .iter()
        .map(|colour| {
            image::open(format!(
                "src/oxen/rendering/sprites/laser_sources/north/{colour}.png"
            ))
            .unwrap()
            .to_rgb8()
        })
        .collect::<Vec<RgbImage>>()
        .try_into()
        .unwrap();
    pub static ref LASER_SOURCES_EAST: [RgbImage; 4] = STR_AGENT_COLOURS
        .iter()
        .map(|colour| {
            let name = format!("src/oxen/rendering/sprites/laser_sources/east/{colour}.png");
            image::open(name).unwrap().to_rgb8()
        })
        .collect::<Vec<RgbImage>>()
        .try_into()
        .unwrap();
    pub static ref LASER_SOURCES_SOUTH: [RgbImage; 4] = STR_AGENT_COLOURS
        .iter()
        .map(|colour| {
            image::open(format!(
                "src/oxen/rendering/sprites/laser_sources/south/{colour}.png"
            ))
            .unwrap()
            .to_rgb8()
        })
        .collect::<Vec<RgbImage>>()
        .try_into()
        .unwrap();
    pub static ref LASER_SOURCES_WEST: [RgbImage; 4] = STR_AGENT_COLOURS
        .iter()
        .map(|colour| {
            image::open(format!(
                "src/oxen/rendering/sprites/laser_sources/west/{colour}.png"
            ))
            .unwrap()
            .to_rgb8()
        })
        .collect::<Vec<RgbImage>>()
        .try_into()
        .unwrap();
    pub static ref FLOOR: RgbImage =
        RgbImage::from_pixel(TILE_SIZE, TILE_SIZE, BACKGROUND_GREY);
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
                    // return Rgba([*colour, *colour, *colour, 255u8]);
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
