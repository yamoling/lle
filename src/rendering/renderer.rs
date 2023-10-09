use image::{GenericImage, Rgb, RgbImage, RgbaImage};

use super::{sprites, TileVisitor, AGENT_COLOURS, BLACK, GRID_GREY};
use crate::{
    tiles::{Direction, Gem, Laser, LaserSource},
    World,
};

use super::{BACKGROUND_GREY, TILE_SIZE};

pub struct VisitorData<'a> {
    x: u32,
    y: u32,
    frame: &'a mut RgbImage,
}

#[derive(Clone)]
pub struct Renderer {
    static_frame: RgbImage,
    pixel_width: u32,
    pixel_height: u32,
}

impl Renderer {
    pub fn new(world: &World) -> Self {
        let pixel_width = world.width() as u32 * TILE_SIZE + 1;
        let pixel_height = world.height() as u32 * TILE_SIZE + 1;
        let mut renderer = Self {
            static_frame: image::RgbImage::new(pixel_width, pixel_height),
            pixel_width,
            pixel_height,
        };
        renderer.static_rendering(world);
        renderer
    }

    /// Draw the floor, walls, laser sources, start and exit tiles.
    fn static_rendering(&mut self, world: &World) {
        // Floor
        self.static_frame.fill(BACKGROUND_GREY.0[0]);
        // Walls
        for pos in world.walls() {
            let x = pos.1 as u32 * TILE_SIZE;
            let y = pos.0 as u32 * TILE_SIZE;
            self.static_frame
                .copy_from(&(*sprites::WALL), x, y)
                .unwrap();
        }

        // Laser sources
        for (pos, source) in world.laser_sources() {
            let x = pos.1 as u32 * TILE_SIZE;
            let y = pos.0 as u32 * TILE_SIZE;
            self.draw_laser_source(source, x, y);
        }

        // Start
        for (id, pos) in world.starts() {
            let x = pos.1 as u32 * TILE_SIZE;
            let y = pos.0 as u32 * TILE_SIZE;
            draw_rectangle(
                &mut self.static_frame,
                x,
                y,
                TILE_SIZE,
                TILE_SIZE,
                AGENT_COLOURS[id],
                3,
            );
        }

        // Exit
        for (&(i, j), _) in world.exits() {
            let x = j as u32 * TILE_SIZE;
            let y = i as u32 * TILE_SIZE;
            draw_rectangle(&mut self.static_frame, x, y, TILE_SIZE, TILE_SIZE, BLACK, 3);
        }

        // Void
        for pos in world.void_positions() {
            let x = pos.1 as u32 * TILE_SIZE;
            let y = pos.0 as u32 * TILE_SIZE;
            // copy the void image to the static one
            add_transparent_image(&mut self.static_frame, &sprites::VOID, x, y);
            // self.static_frame
            //     .copy_from(&(*sprites::VOID), x, y)
            //     .unwrap();
        }
    }

    fn draw_laser_source(&mut self, laser_source: &LaserSource, x: u32, y: u32) {
        let agent_id = laser_source.agent_id();
        let source_sprite = match laser_source.direction() {
            Direction::North => &sprites::LASER_SOURCES_NORTH[agent_id],
            Direction::East => &sprites::LASER_SOURCES_EAST[agent_id],
            Direction::South => &sprites::LASER_SOURCES_SOUTH[agent_id],
            Direction::West => &sprites::LASER_SOURCES_WEST[agent_id],
        };
        self.static_frame.copy_from(source_sprite, x, y).unwrap();
    }

    pub fn update(&self, world: &World) -> RgbImage {
        let mut frame = self.static_frame.clone();
        for (pos, laser) in world.lasers() {
            let mut data = VisitorData {
                x: pos.1 as u32 * TILE_SIZE,
                y: pos.0 as u32 * TILE_SIZE,
                frame: &mut frame,
            };
            if laser.is_on() {
                let agent_id = laser.agent_id();
                let laser_sprite = match laser.direction() {
                    Direction::North | Direction::South => &sprites::VERTICAL_LASERS[agent_id],
                    Direction::East | Direction::West => &sprites::HORIZONTAL_LASERS[agent_id],
                };
                add_transparent_image(data.frame, laser_sprite, data.x, data.y);
            }
            // Draw the tile below the laser
            laser.wrapped().accept(self, &mut data);
        }
        for (pos, gem) in world.gems() {
            let x = pos.1 as u32 * TILE_SIZE;
            let y = pos.0 as u32 * TILE_SIZE;
            if !gem.is_collected() {
                add_transparent_image(&mut frame, &sprites::GEM, x, y);
            }
        }
        for (id, pos) in world.agents_positions().iter().enumerate() {
            let x = pos.1 as u32 * TILE_SIZE;
            let y = pos.0 as u32 * TILE_SIZE;
            add_transparent_image(&mut frame, &sprites::AGENTS[id], x, y);
        }
        draw_grid(&mut frame);
        frame
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
    img.copy_from(&horizontal_line, x, y).unwrap();
    img.copy_from(&horizontal_line, x, y + height - thickness + 1)
        .unwrap();
    img.copy_from(&vertical_line, x, y).unwrap();
    img.copy_from(&vertical_line, x + width - thickness + 1, y)
        .unwrap();
}

impl TileVisitor for Renderer {
    fn visit_gem(&self, gem: &Gem, data: &mut VisitorData) {
        if !gem.is_collected() {
            add_transparent_image(data.frame, &sprites::GEM, data.x, data.y);
        }
    }

    fn visit_laser(&self, laser: &Laser, data: &mut VisitorData) {
        if laser.is_on() {
            let agent_id = laser.agent_id();
            let laser_sprite = match laser.direction() {
                Direction::North | Direction::South => &sprites::VERTICAL_LASERS[agent_id],
                Direction::East | Direction::West => &sprites::HORIZONTAL_LASERS[agent_id],
            };
            add_transparent_image(data.frame, laser_sprite, data.x, data.y);
        }
        // Draw the tile below the laser
        laser.wrapped().accept(self, data);
        // The below tile should draw the agent
    }
}
