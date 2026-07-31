use image::{GenericImage, Rgb, RgbImage, RgbaImage};
use itertools::izip;

use super::{BLACK, GRID_GREY, TileVisitor, sprites};
use crate::{
    core::World,
    tiles::{Button, CardinalDirection, Gem, Laser, LaserSource, Lift, VerticalDirection},
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
        for (pos, lift) in world.lifts() {
            let mut data = VisitorData {
                x: pos.x() as u32 * TILE_SIZE,
                y: pos.y() as u32 * TILE_SIZE,
                frame: &mut frame_stack[pos.z() as usize],
            };
            self.visit_lift(lift, &mut data);
        }
        for (pos, button) in world.buttons() {
            let mut data = VisitorData {
                x: pos.x() as u32 * TILE_SIZE,
                y: pos.y() as u32 * TILE_SIZE,
                frame: &mut frame_stack[pos.z() as usize],
            };
            self.visit_button(button, &mut data);
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

/// Corner offset (in pixels) at which the ~14px `AGENT_LOCK` badge is drawn
/// within a 32px tile, keeping it fully inside the tile with a small margin.
const BADGE_OFFSET: u32 = TILE_SIZE - 14 - 2;

/// Recolor a white-on-transparent mask sprite by multiplying its RGB channels
/// by `color`, preserving per-pixel alpha. Used for `Lift`/`Button` sprites,
/// whose `group_id` is unbounded (unlike the 4 fixed agent colors), so their
/// color can't be baked into a fixed set of sprite files the way
/// `sprites::AGENTS` is.
fn tint_image(sprite: &RgbaImage, color: Rgb<u8>) -> RgbaImage {
    RgbaImage::from_fn(sprite.width(), sprite.height(), |x, y| {
        let p = sprite.get_pixel(x, y).0;
        image::Rgba([
            (p[0] as u32 * color.0[0] as u32 / 255) as u8,
            (p[1] as u32 * color.0[1] as u32 / 255) as u8,
            (p[2] as u32 * color.0[2] as u32 / 255) as u8,
            p[3],
        ])
    })
}

/// Fixed colors for the per-agent restriction badge, indexed by `AgentId`,
/// matching the order of the `agents/{red,blue,green,yellow}.png` sprites.
/// Unlike `group_color`, this is a fixed 4-entry table since agent count is
/// bounded the same way `sprites::AGENTS` is.
const AGENT_COLORS: [Rgb<u8>; 4] = [
    Rgb([220, 20, 20]),
    Rgb([30, 80, 220]),
    Rgb([30, 160, 60]),
    Rgb([220, 190, 20]),
];

/// A deterministic, visually distinct color for a given `group_id`, obtained by
/// rotating the hue by the golden angle each time so consecutive group ids don't
/// look alike.
fn group_color(group_id: usize) -> Rgb<u8> {
    let hue = (group_id as f32 * 137.508) % 360.0;
    hsv_to_rgb(hue, 0.65, 0.9)
}

fn hsv_to_rgb(h: f32, s: f32, v: f32) -> Rgb<u8> {
    let c = v * s;
    let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
    let m = v - c;
    let (r, g, b) = match h as u32 {
        0..=59 => (c, x, 0.0),
        60..=119 => (x, c, 0.0),
        120..=179 => (0.0, c, x),
        180..=239 => (0.0, x, c),
        240..=299 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    Rgb([
        (((r + m) * 255.0).round()) as u8,
        (((g + m) * 255.0).round()) as u8,
        (((b + m) * 255.0).round()) as u8,
    ])
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
                CardinalDirection::North | CardinalDirection::South => {
                    &sprites::VERTICAL_LASERS[agent_id]
                }
                CardinalDirection::East | CardinalDirection::West => {
                    &sprites::HORIZONTAL_LASERS[agent_id]
                }
            };
            add_transparent_image(data.frame, laser_sprite, data.x, data.y);
        }
        // Draw the tile below the laser
        laser.wrapped().accept(self, data);
    }

    fn visit_laser_source(&self, source: &LaserSource, data: &mut VisitorData) {
        let agent_id = source.agent_id();
        let source_sprite = match source.direction() {
            CardinalDirection::North => &sprites::LASER_SOURCES_NORTH[agent_id],
            CardinalDirection::East => &sprites::LASER_SOURCES_EAST[agent_id],
            CardinalDirection::South => &sprites::LASER_SOURCES_SOUTH[agent_id],
            CardinalDirection::West => &sprites::LASER_SOURCES_WEST[agent_id],
        };
        data.frame.copy_from(source_sprite, data.x, data.y).unwrap();
    }

    fn visit_lift(&self, lift: &Lift, data: &mut VisitorData) {
        let sprite = match lift.direction() {
            VerticalDirection::Up => &*sprites::LIFT_UP,
            VerticalDirection::Down => &*sprites::LIFT_DOWN,
        };
        let tinted = tint_image(sprite, group_color(lift.group_id()));
        add_transparent_image(data.frame, &tinted, data.x, data.y);
        if let Some(agent_id) = lift.authorized_agent_id() {
            let badge = tint_image(&sprites::AGENT_LOCK, AGENT_COLORS[agent_id]);
            add_transparent_image(
                data.frame,
                &badge,
                data.x + BADGE_OFFSET,
                data.y + BADGE_OFFSET,
            );
        }
    }

    fn visit_button(&self, button: &Button, data: &mut VisitorData) {
        let sprite = if button.agent().is_some() {
            &*sprites::BUTTON_PRESSED
        } else {
            &*sprites::BUTTON_IDLE
        };
        let tinted = tint_image(sprite, group_color(button.group_id()));
        add_transparent_image(data.frame, &tinted, data.x, data.y);
        if let Some(agent_id) = button.authorized_agent_id() {
            let badge = tint_image(&sprites::AGENT_LOCK, AGENT_COLORS[agent_id]);
            add_transparent_image(
                data.frame,
                &badge,
                data.x + BADGE_OFFSET,
                data.y + BADGE_OFFSET,
            );
        }
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

    #[test]
    fn lift_and_button_are_rendered() {
        let world = World::try_from("S0 . TU0\nB0 .  X").unwrap();
        let renderer = Renderer::new(&world);
        let image = renderer.update(&world);

        // A plain floor tile is left as the untouched background fill.
        let floor_pixel = *image.get_pixel(TILE_SIZE + TILE_SIZE / 2, TILE_SIZE / 2);

        // Lift at (row 0, col 2): the up-arrow sprite covers the tile center.
        let lift_pixel = *image.get_pixel(2 * TILE_SIZE + TILE_SIZE / 2, TILE_SIZE / 2);
        assert_ne!(lift_pixel, floor_pixel);

        // Button at (row 1, col 0), unoccupied: only the idle ring is drawn
        // (the tile center is transparent), so sample a pixel on the ring itself.
        let button_pixel = *image.get_pixel(TILE_SIZE / 2, TILE_SIZE + 5);
        assert_ne!(button_pixel, floor_pixel);
    }

    #[test]
    fn lift_and_button_restriction_badge_is_rendered() {
        // Same group_id (0), same shape/direction — only the `A0` suffix
        // restricts the tile to agent 0. The badge should be the only
        // difference between the two renders.
        let unrestricted = World::try_from("S0 . TU0\nB0 .  X").unwrap();
        let restricted = World::try_from("S0 . TU0A0\nB0A0 .  X").unwrap();

        let unrestricted_image = Renderer::new(&unrestricted).update(&unrestricted);
        let restricted_image = Renderer::new(&restricted).update(&restricted);

        const BADGE_OFFSET: u32 = TILE_SIZE - 14 - 2;
        // Offset of an opaque pixel inside the badge sprite itself (its
        // top-left corner is transparent, so sampling BADGE_OFFSET alone
        // would land on background/sprite-underneath, not the badge).
        const BADGE_INNER: u32 = 7;

        // Lift at (row 0, col 2).
        let lift_x = 2 * TILE_SIZE + BADGE_OFFSET + BADGE_INNER;
        let lift_y = BADGE_OFFSET + BADGE_INNER;
        assert_ne!(
            *restricted_image.get_pixel(lift_x, lift_y),
            *unrestricted_image.get_pixel(lift_x, lift_y)
        );

        // Button at (row 1, col 0).
        let button_x = BADGE_OFFSET + BADGE_INNER;
        let button_y = TILE_SIZE + BADGE_OFFSET + BADGE_INNER;
        assert_ne!(
            *restricted_image.get_pixel(button_x, button_y),
            *unrestricted_image.get_pixel(button_x, button_y)
        );
    }
}
