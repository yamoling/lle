use image::{GenericImage, Rgb, RgbImage, RgbaImage};
use itertools::izip;

use super::{BLACK, GRID_GREY, TileVisitor, sprites};
use crate::{
    core::World,
    tiles::{Direction, Gem, Laser, LaserSource},
};

use super::{BACKGROUND_GREY, TILE_SIZE};

pub struct VisitorData<'a> {
    x: u32,
    y: u32,
    frame: &'a mut RgbImage,
}

#[derive(Clone, Copy)]
pub enum PanelFmt {
    /// The panels are displayed in a grid format, with each panel representing a layer of the world. The panels are arranged in a single row, with the first panel representing the bottom layer and the last panel representing the top layer.
    Grid(usize, usize),
    /// The panels are displayed in a stacked format, with each panel representing a layer of the world. The panels are arranged on top of each other, with the first panel representing the bottom layer and the last panel representing the top layer.
    VStack,
    /// The panels are displayed in a horizontal format, with each panel representing a layer of the world. The panels are arranged in a single column, with the first panel representing the bottom layer and the last panel representing the top layer.
    HStack,
}

///The Renderer struct is responsible for rendering the world as an image. It has a static frame which contains the floor, walls, laser sources, start and exit tiles. The dynamic elements such as lasers, gems and agents are rendered on top of the static frame in the update method.
/// The addition of vector of RgbImage allows us to have a separate static frame for each layer of the world, give us the ability to render each layer independently and then diplay them in any ways that we want.
#[derive(Clone)]
pub struct Renderer {
    static_frame: Vec<RgbImage>,
    pixel_width: u32,
    pixel_height: u32,
    panel_fmt: PanelFmt,
}

impl Renderer {
    pub fn new(core: &World) -> Self {
        let pixel_width = core.width() as u32 * TILE_SIZE + 1;
        let pixel_height = core.height() as u32 * TILE_SIZE + 1;
        let mut renderer = Self {
            static_frame: vec![image::RgbImage::new(pixel_width, pixel_height); core.layers()],
            pixel_width,
            pixel_height,
            panel_fmt: PanelFmt::VStack,
        };
        renderer.static_rendering(core);
        info!(target: LOG, "Initialized renderer with dimensions {}x{} and {} layers", pixel_width, pixel_height, core.layers());
        renderer
    }

    /// Draw the floor, walls, laser sources, start and exit tiles.
    fn static_rendering(&mut self, world: &World) {
        // Floor
        self.static_frame.iter_mut().for_each(|frame| {
            frame.fill(BACKGROUND_GREY.0[0]);
        });
        // Walls
        for pos in world.walls() {
            let x = pos.x() as u32 * TILE_SIZE;
            let y = pos.y() as u32 * TILE_SIZE;
            let z = pos.z() as usize;
            self.static_frame[z]
                .copy_from(&(*sprites::WALL), x, y)
                .unwrap();
        }

        // Exit
        for pos in world.exits_positions() {
            let x = pos.x() as u32 * TILE_SIZE;
            let y = pos.y() as u32 * TILE_SIZE;
            let z = pos.z() as usize;
            draw_rectangle(
                &mut self.static_frame[z],
                x,
                y,
                TILE_SIZE,
                TILE_SIZE,
                BLACK,
                3,
            );
        }

        // Void
        for pos in world.void_positions() {
            let x = pos.x() as u32 * TILE_SIZE;
            let y = pos.y() as u32 * TILE_SIZE;
            let z = pos.z() as usize;
            // copy the void image to the static one
            add_transparent_image(&mut self.static_frame[z], &sprites::VOID, x, y);
        }
    }

    pub fn update(&self, world: &World) -> RgbImage {
        let mut frame_stack = self.static_frame.clone();
        for (pos, laser) in world.lasers() {
            let mut data = VisitorData {
                x: pos.x() as u32 * TILE_SIZE,
                y: pos.y() as u32 * TILE_SIZE,
                frame: &mut frame_stack[pos.z() as usize],
            };
            self.visit_laser(laser, &mut data);
        }
        for (pos, gem) in izip!(world.gems_positions(), world.gems()) {
            let mut data = VisitorData {
                x: pos.x() as u32 * TILE_SIZE,
                y: pos.y() as u32 * TILE_SIZE,
                frame: &mut frame_stack[pos.z() as usize],
            };
            self.visit_gem(&gem, &mut data);
        }
        for (id, pos) in world.agents_positions().iter().enumerate() {
            let x = pos.x() as u32 * TILE_SIZE;
            let y = pos.y() as u32 * TILE_SIZE;
            add_transparent_image(
                &mut frame_stack[pos.z() as usize],
                &sprites::AGENTS[id],
                x,
                y,
            );
        }
        for (pos, source) in world.sources() {
            let mut data = VisitorData {
                x: pos.x() as u32 * TILE_SIZE,
                y: pos.y() as u32 * TILE_SIZE,
                frame: &mut frame_stack[pos.z() as usize],
            };
            self.visit_laser_source(source, &mut data);
        }
        frame_stack.iter_mut().for_each(|frame| {
            draw_grid(frame);
        });
        return self.concate_single_image(frame_stack);
    }

    fn concate_single_image(&self, frame_stack: Vec<RgbImage>) -> RgbImage {
        let (resized_width, resized_height) = match self.panel_fmt {
            PanelFmt::Grid(_, _) => todo!(),
            PanelFmt::VStack => (
                self.pixel_width,
                self.pixel_height * frame_stack.len() as u32 + frame_stack.len() as u32 - 1,
            ),
            PanelFmt::HStack => (
                self.pixel_width * frame_stack.len() as u32 + frame_stack.len() as u32 - 1,
                self.pixel_height,
            ),
        };
        let mut panel = RgbImage::new(resized_width, resized_height);
        for (i, frame) in frame_stack.iter().enumerate() {
            let (x_offset, y_offset) = match self.panel_fmt {
                PanelFmt::Grid(_, _) => todo!(),
                PanelFmt::VStack => (0, i as u32 * (self.pixel_height + 1)),
                PanelFmt::HStack => (i as u32 * (self.pixel_width + 1), 0),
            };
            panel.copy_from(frame, x_offset, y_offset).unwrap();
        }
        panel
    }
    pub fn pixel_width(&self) -> u32 {
        self.pixel_width * self.stack_width() + self.stack_width() - 1 // stack_width - 1 is the spacing between panels
    }

    pub fn pixel_height(&self) -> u32 {
        self.pixel_height * self.stack_height() + self.stack_height() - 1 // stack_height - 1 is the spacing between panels
    }

    fn stack_width(&self) -> u32 {
        match self.panel_fmt {
            PanelFmt::Grid(cols, _) => cols as u32,
            PanelFmt::VStack => 1,
            PanelFmt::HStack => self.static_frame.len() as u32,
        }
    }

    fn stack_height(&self) -> u32 {
        match self.panel_fmt {
            PanelFmt::Grid(_, rows) => rows as u32,
            PanelFmt::VStack => self.static_frame.len() as u32, // ask yannick
            PanelFmt::HStack => 1,
        }
    }

    pub fn set_panel_fmt(&mut self, fmt: PanelFmt) {
        self.panel_fmt = fmt;
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
    }

    fn visit_laser_source(&self, source: &LaserSource, data: &mut VisitorData) {
        let agent_id = source.agent_id();
        let source_sprite = match source.direction() {
            Direction::North => &sprites::LASER_SOURCES_NORTH[agent_id],
            Direction::East => &sprites::LASER_SOURCES_EAST[agent_id],
            Direction::South => &sprites::LASER_SOURCES_SOUTH[agent_id],
            Direction::West => &sprites::LASER_SOURCES_WEST[agent_id],
        };
        data.frame.copy_from(source_sprite, data.x, data.y).unwrap();
    }
}

#[cfg(test)]
mod test_renderer {
    use crate::{Renderer, World, rendering::TILE_SIZE};

    #[test]
    fn pixel_dimensions() {
        let world = World::try_from("S0 . X").unwrap();
        let renderer = Renderer::new(&world);
        assert_eq!(TILE_SIZE * world.width() as u32 + 1, renderer.pixel_width());
        assert_eq!(
            TILE_SIZE * world.height() as u32 + 1,
            renderer.pixel_height()
        );
    }
}
