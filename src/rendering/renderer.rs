use image::{GenericImage, Rgb, RgbImage, RgbaImage};

use super::{sprites, TileVisitor, AGENT_COLOURS, BLACK, GRID_GREY};
use crate::tiles::{Direction, Gem, Laser, LaserSource, Start, Tile};

use super::{BACKGROUND_GREY, TILE_SIZE};

#[derive(Clone)]
pub struct Renderer {
    static_frame: RgbImage,
    frame: RgbImage,
    pixel_width: u32,
    pixel_height: u32,
}

impl Renderer {
    pub fn new<'a>(
        width: u32,
        height: u32,
        _tiles: impl Iterator<Item = ((u32, u32), &'a Box<dyn Tile>)>,
    ) -> Self {
        let pixel_width: u32 = width * TILE_SIZE + 1;
        let pixel_height: u32 = height * TILE_SIZE + 1;
        let mut static_frame = image::RgbImage::new(pixel_width, pixel_height);
        static_frame.fill(BACKGROUND_GREY.0[0]);
        // static_initial_rendering(&mut static_frame, tiles);

        Self {
            frame: static_frame.clone(),
            static_frame,
            pixel_width,
            pixel_height,
        }
    }

    pub fn update<'a>(
        &mut self,
        tiles: impl Iterator<Item = ((u32, u32), &'a Box<dyn Tile>)>,
    ) -> RgbImage {
        let mut screen = self.static_frame.clone();
        for ((i, j), tile) in tiles {
            let x = j * TILE_SIZE;
            let y = i * TILE_SIZE;
            tile.accept(self, x, y);
            if let Some(agent) = tile.agent() {
                add_transparent_image(&mut screen, &sprites::AGENTS[agent as usize], x, y);
            }
        }
        draw_grid(&mut screen);
        screen
    }

    pub fn pixel_width(&self) -> u32 {
        self.pixel_width
    }

    pub fn pixel_height(&self) -> u32 {
        self.pixel_height
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

impl TileVisitor for Renderer {
    fn visit_start(&mut self, start: &Start, x: u32, y: u32) {
        draw_rectangle(
            &mut self.frame,
            x,
            y,
            TILE_SIZE,
            TILE_SIZE,
            AGENT_COLOURS[start.agent_id() as usize],
            2,
        );
    }

    fn visit_exit(&mut self, x: u32, y: u32) {
        draw_rectangle(&mut self.frame, x, y, TILE_SIZE, TILE_SIZE, BLACK, 2);
        //self.frame.copy_from(&(*sprites::EXIT), x, y).unwrap()
    }

    fn visit_wall(&mut self, x: u32, y: u32) {
        self.frame.copy_from(&(*sprites::WALL), x, y).unwrap();
    }

    fn visit_gem(&mut self, gem: &Gem, x: u32, y: u32) {
        self.frame.copy_from(&(*sprites::FLOOR), x, y).unwrap();
        if !gem.is_collected() {
            add_transparent_image(&mut self.frame, &sprites::GEM, x, y);
        }
    }

    fn visit_laser(&mut self, laser: &Laser, x: u32, y: u32) {
        if laser.is_on() {
            let agent_id = laser.agent_id() as usize;
            let laser_sprite = match laser.direction() {
                Direction::North | Direction::South => &sprites::HORIZONTAL_LASERS[agent_id],
                Direction::East | Direction::West => &sprites::HORIZONTAL_LASERS[agent_id],
            };
            add_transparent_image(&mut self.frame, laser_sprite, x, y);
        }
        // Draw the tile below the laser
        laser.wrapped().accept(self, x, y);
    }

    fn visit_laser_source(&mut self, laser_source: &LaserSource, x: u32, y: u32) {
        let agent_id = laser_source.agent_id() as usize;
        let source_sprite = match laser_source.direction() {
            Direction::North => &sprites::LASER_SOURCES_NORTH[agent_id],
            Direction::East => &sprites::LASER_SOURCES_EAST[agent_id],
            Direction::South => &sprites::LASER_SOURCES_SOUTH[agent_id],
            Direction::West => &sprites::LASER_SOURCES_WEST[agent_id],
        };
        self.frame.copy_from(source_sprite, x, y).unwrap();
    }
}
