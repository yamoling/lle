use image::{GenericImage, Rgb, RgbImage, RgbaImage};
use pyo3::prelude::*;

use super::{sprites, AGENT_COLOURS, BLACK, GRID_GREY};
use crate::tiles::{laser::Direction, laser_source::LaserSource, Tile, TileType};

use super::{BACKGROUND_GREY, TILE_SIZE};

pub struct Renderer {
    screen: RgbImage,
}

impl Renderer {
    pub fn new<'a>(
        width: u32,
        height: u32,
        tiles: impl Iterator<Item = ((u32, u32), &'a Tile)>,
    ) -> Self {
        let pixel_width: u32 = width * TILE_SIZE + 1;
        let pixel_height: u32 = height * TILE_SIZE + 1;
        let mut screen = image::RgbImage::new(pixel_width, pixel_height);
        static_initial_rendering(&mut screen, tiles);

        Self { screen }
    }

    pub fn update<'a>(&mut self, tiles: impl Iterator<Item = ((u32, u32), &'a Tile)>) -> RgbImage {
        let mut screen = self.screen.clone();
        for ((i, j), tile) in tiles {
            let x = j * TILE_SIZE;
            let y = i * TILE_SIZE;
            draw_tile(&mut screen, x, y, tile);
            if let Some(agent) = tile.agent() {
                add_transparent_image(
                    &mut screen,
                    &sprites::AGENTS[agent.borrow().num() as usize],
                    x,
                    y,
                );
            }
        }
        draw_grid(&mut screen);
        screen
    }
}

fn draw_tile(screen: &mut RgbImage, x: u32, y: u32, tile: &Tile) {
    match tile.tile_type() {
        // All these case have already been covered in the static rendering
        TileType::Floor | TileType::LaserSource { .. } | TileType::Wall => {}
        TileType::Exit { .. } => draw_rectangle(screen, x, y, TILE_SIZE, TILE_SIZE, BLACK, 2),
        TileType::Start { agent_num } => {
            draw_rectangle(
                screen,
                x,
                y,
                TILE_SIZE,
                TILE_SIZE,
                AGENT_COLOURS[*agent_num as usize],
                2,
            );
        }
        TileType::Gem { collected } => {
            screen.copy_from(&(*sprites::FLOOR), x, y).unwrap();
            if !collected {
                add_transparent_image(screen, &sprites::GEM, x, y);
            }
        }
        TileType::Laser(laser) => {
            if laser.is_on() {
                let laser_sprite = match laser.direction() {
                    Direction::North | Direction::South => {
                        &sprites::HORIZONTAL_LASERS[laser.agent_num() as usize]
                    }
                    Direction::East | Direction::West => {
                        &sprites::HORIZONTAL_LASERS[laser.agent_num() as usize]
                    }
                };
                add_transparent_image(screen, laser_sprite, x, y);
            }
            // Draw the tile below the laser
            draw_tile(screen, x, y, laser.wrapped());
        }
    }
}

fn draw_static_tile(screen: &mut RgbImage, x: u32, y: u32, tile: &Tile) {
    match tile.tile_type() {
        TileType::LaserSource(LaserSource {
            agent_num,
            direction,
        }) => {
            let source_sprite = match direction {
                Direction::North => &sprites::LASER_SOURCES_NORTH[*agent_num as usize],
                Direction::East => &sprites::LASER_SOURCES_EAST[*agent_num as usize],
                Direction::South => &sprites::LASER_SOURCES_SOUTH[*agent_num as usize],
                Direction::West => &sprites::LASER_SOURCES_WEST[*agent_num as usize],
            };
            screen.copy_from(source_sprite, x, y).unwrap();
        }
        TileType::Wall => screen.copy_from(&(*sprites::WALL), x, y).unwrap(),
        TileType::Laser(laser) => {
            draw_static_tile(screen, x, y, laser.wrapped());
        }
        // Floor and Gem are already covered by the background
        TileType::Floor | TileType::Gem { .. } => {}
        // Handled dynamically at each
        TileType::Exit { .. } => screen.copy_from(&(*sprites::FINISH), x, y).unwrap(),
        TileType::Start { .. } => {}
    }
}

fn static_initial_rendering<'a>(
    screen: &mut RgbImage,
    tiles: impl Iterator<Item = ((u32, u32), &'a Tile)>,
) {
    screen.fill(BACKGROUND_GREY.0[0]);

    for ((i, j), tile) in tiles {
        let x = j * TILE_SIZE;
        let y = i * TILE_SIZE;
        draw_static_tile(screen, x, y, tile);
    }
}

fn draw_grid(img: &mut RgbImage) {
    let width = img.width();
    let height = img.height();
    let horizontal_line = RgbImage::from_pixel(width, 1, GRID_GREY);
    let vertical_line = RgbImage::from_pixel(1, height, GRID_GREY);
    for i in (0..height).step_by(TILE_SIZE as usize) {
        img.copy_from(&horizontal_line, 0, i).unwrap();
    }
    for j in (0..width).step_by(TILE_SIZE as usize) {
        img.copy_from(&vertical_line, j, 0).unwrap();
    }
}

fn add_transparent_image(
    background: &mut RgbImage,
    sprite: &RgbaImage,
    offset_x: u32,
    offset_y: u32,
) {
    // Iterate over each pixel in the RGBA image and blend it with the corresponding pixel in the RGB image
    for (x, y, pixel) in sprite.enumerate_pixels() {
        let rgba = pixel.0;
        let rgb = &mut background.get_pixel_mut(x + offset_x, y + offset_y).0;

        // Blend the two pixels using the alpha value of the RGBA pixel
        let alpha = rgba[3] as f32 / 255.0;
        rgb[0] = ((1.0 - alpha) * rgb[0] as f32 + alpha * rgba[0] as f32) as u8;
        rgb[1] = ((1.0 - alpha) * rgb[1] as f32 + alpha * rgba[1] as f32) as u8;
        rgb[2] = ((1.0 - alpha) * rgb[2] as f32 + alpha * rgba[2] as f32) as u8;
    }
}

fn draw_rectangle(
    img: &mut RgbImage,
    x: u32,
    y: u32,
    width: u32,
    height: u32,
    color: Rgb<u8>,
    thickness: u32,
) {
    let horizontal_line = RgbImage::from_pixel(width, thickness, color);
    let vertical_line = RgbImage::from_pixel(thickness, height, color);
    img.copy_from(&horizontal_line, x, y + thickness - 1)
        .unwrap();
    img.copy_from(&horizontal_line, x, y + height - thickness)
        .unwrap();
    img.copy_from(&vertical_line, x + thickness - 1, y).unwrap();
    img.copy_from(&vertical_line, x + width - thickness, y)
        .unwrap();
}
