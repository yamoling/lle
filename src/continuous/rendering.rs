//! Rendering of a continuous world, at the scale of LLE: [`TILE_SIZE`] (32) pixels per cell, so
//! that a frame has the shape `(height * 32, width * 32, 3)`.
//!
//! - The **static background** holds the floor, the walls, the exits, the voids, the laser
//!   sources and the grid. It is rendered lazily, by the first call to
//!   [`ContinuousWorld::get_image`], and cached: copies of the world share it, and changing the
//!   colour of a source discards it. Nothing else ever touches it, so a world that is never
//!   rendered never allocates an image.
//! - The **dynamic layer** is drawn on a copy of the background at each frame: the gems that are
//!   not collected, the current beams (after cuts) as anti-aliased lines at least 3 pixels wide,
//!   and the alive agents as anti-aliased discs in their colour, with a short velocity indicator
//!   in a darker shade when they move, and a white outline once they have arrived. Dead agents
//!   are not drawn.
//!
//! The colours of the agents and the beams are the accent colours of the agent sprites
//! ([`sprites::agent_colour`]).
use std::{
    fmt::Debug,
    sync::{Arc, OnceLock},
};

use image::{GenericImage, Rgb, RgbImage, RgbaImage};

use super::{ContinuousWorld, geometry::*, observations::beam_rectangle};
use crate::shared::{
    CardinalDirection, Position,
    sprites::{self, BACKGROUND_GREY, BLACK, GRID_GREY, TILE_SIZE},
};

/// The number of pixels per cell, as a float.
const PX: f64 = TILE_SIZE as f64;
/// The minimal width of a beam, in pixels.
const MIN_BEAM_WIDTH: f64 = 3.0;
/// The thickness of the outline of the exits, in pixels.
const EXIT_THICKNESS: u32 = 3;
/// The width of the white outline of an arrived agent, in pixels.
const OUTLINE_WIDTH: f64 = 2.0;
/// The width of the velocity indicator, in pixels.
const INDICATOR_WIDTH: f64 = 2.0;
const WHITE: Rgb<u8> = Rgb([255, 255, 255]);

/// The lazily rendered static background of a world. Cloning it shares the image.
#[derive(Clone, Default)]
pub(super) struct Background(OnceLock<Arc<RgbImage>>);

impl Background {
    /// Discard the cached image, to render it again at the next frame.
    pub(super) fn invalidate(&mut self) {
        self.0 = OnceLock::new();
    }
    pub(super) fn is_rendered(&self) -> bool {
        self.0.get().is_some()
    }
}

impl Debug for Background {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("Background")
            .field(&self.is_rendered())
            .finish()
    }
}

impl ContinuousWorld {
    /// The width and the height of a frame, in pixels.
    pub fn image_dimensions(&self) -> (u32, u32) {
        (
            self.width as u32 * TILE_SIZE,
            self.height as u32 * TILE_SIZE,
        )
    }

    /// Render the current frame (see the module documentation). The static background is
    /// rendered and cached on the first call.
    ///
    /// @ai-generated
    pub fn get_image(&self) -> RgbImage {
        let background = self
            .background
            .0
            .get_or_init(|| Arc::new(self.render_background()));
        let mut frame = RgbImage::clone(background);
        for (pos, &collected) in self.gems.iter().zip(&self.state.gems_collected) {
            if !collected {
                let (x, y) = cell_origin(pos);
                add_transparent_image(&mut frame, &sprites::GEM, x, y);
            }
        }
        self.draw_beams(&mut frame);
        self.draw_agents(&mut frame);
        frame
    }

    /// Whether the static background has been rendered (and cached).
    #[cfg(test)]
    pub(crate) fn has_rendered_background(&self) -> bool {
        self.background.is_rendered()
    }

    /// The floor, the walls, the exits, the voids, the laser sources and the grid.
    ///
    /// @ai-generated
    fn render_background(&self) -> RgbImage {
        let (width, height) = self.image_dimensions();
        let mut frame = RgbImage::from_pixel(width, height, BACKGROUND_GREY);
        for pos in self.walls() {
            let (x, y) = cell_origin(pos);
            frame.copy_from(&*sprites::WALL, x, y).unwrap();
        }
        for pos in &self.exits {
            let (x, y) = cell_origin(pos);
            draw_square_outline(&mut frame, x, y, BLACK, EXIT_THICKNESS);
        }
        for pos in &self.voids {
            let (x, y) = cell_origin(pos);
            add_transparent_image(&mut frame, &sprites::VOID, x, y);
        }
        for source in &self.sources {
            let sprite = match source.direction {
                CardinalDirection::North => sprites::laser_source_north(source.colour),
                CardinalDirection::East => sprites::laser_source_east(source.colour),
                CardinalDirection::South => sprites::laser_source_south(source.colour),
                CardinalDirection::West => sprites::laser_source_west(source.colour),
            };
            let (x, y) = cell_origin(&source.position);
            frame.copy_from(sprite, x, y).unwrap();
        }
        for y in (0..height).step_by(TILE_SIZE as usize) {
            for x in 0..width {
                frame.put_pixel(x, y, GRID_GREY);
            }
        }
        for x in (0..width).step_by(TILE_SIZE as usize) {
            for y in 0..height {
                frame.put_pixel(x, y, GRID_GREY);
            }
        }
        frame
    }

    /// Draw the current segment of every enabled beam as an anti-aliased rectangle in the colour
    /// of its laser, `max(beam_width * 32, 3)` pixels wide.
    ///
    /// @ai-generated
    fn draw_beams(&self, frame: &mut RgbImage) {
        let half_width = (self.params.beam_width * PX).max(MIN_BEAM_WIDTH) / 2.0;
        for (beam, segment) in self.beams.iter().zip(self.beam_segments()) {
            let Some(cut) = segment else { continue };
            let origin = [beam.origin[0] * PX, beam.origin[1] * PX];
            let (lo, hi) = beam_rectangle(origin, beam.direction, cut * PX, half_width);
            let colour = sprites::agent_colour(beam.colour);
            for_pixels(frame, lo, hi, |frame, x, y| {
                let coverage = overlap(x as f64, lo[0], hi[0]) * overlap(y as f64, lo[1], hi[1]);
                blend(frame, x, y, colour, coverage);
            });
        }
    }

    /// Draw every alive agent as an anti-aliased disc of radius `agent_radius * 32` in its
    /// colour, with a velocity indicator in a darker shade when it moves, and a white outline
    /// once it has arrived.
    ///
    /// @ai-generated
    fn draw_agents(&self, frame: &mut RgbImage) {
        let radius = self.params.agent_radius * PX;
        for agent in 0..self.n_agents() {
            if !self.state.agents_alive[agent] {
                continue;
            }
            let [x, y] = self.state.positions[agent];
            let centre = [x * PX, y * PX];
            let lo = [centre[0] - radius - 1.0, centre[1] - radius - 1.0];
            let hi = [centre[0] + radius + 1.0, centre[1] + radius + 1.0];
            let colour = sprites::agent_colour(self.colours[agent]);
            for_pixels(frame, lo, hi, |frame, px, py| {
                let d = norm(sub(pixel_centre(px, py), centre));
                blend(frame, px, py, colour, (radius + 0.5 - d).clamp(0.0, 1.0));
            });
            let velocity = self.state.velocities[agent];
            let speed = norm(velocity);
            if speed > 1e-9 {
                let u = [velocity[0] / speed, velocity[1] / speed];
                let a = [
                    centre[0] + u[0] * 0.35 * radius,
                    centre[1] + u[1] * 0.35 * radius,
                ];
                let b = [
                    centre[0] + u[0] * 0.95 * radius,
                    centre[1] + u[1] * 0.95 * radius,
                ];
                let dark = Rgb(colour.0.map(|c| c / 2));
                for_pixels(frame, lo, hi, |frame, px, py| {
                    let d = segment_distance(pixel_centre(px, py), a, b);
                    let coverage = (INDICATOR_WIDTH / 2.0 + 0.5 - d).clamp(0.0, 1.0);
                    blend(frame, px, py, dark, coverage);
                });
            }
            if self.state.agents_arrived[agent] {
                let ring = radius - OUTLINE_WIDTH / 2.0;
                for_pixels(frame, lo, hi, |frame, px, py| {
                    let d = norm(sub(pixel_centre(px, py), centre));
                    let coverage = (OUTLINE_WIDTH / 2.0 + 0.5 - (d - ring).abs()).clamp(0.0, 1.0);
                    blend(frame, px, py, WHITE, coverage);
                });
            }
        }
    }
}

/// The top-left pixel of `pos`.
fn cell_origin(pos: &Position) -> (u32, u32) {
    (pos.j as u32 * TILE_SIZE, pos.i as u32 * TILE_SIZE)
}

fn pixel_centre(x: u32, y: u32) -> [f64; 2] {
    [x as f64 + 0.5, y as f64 + 0.5]
}

/// The length of the overlap of the pixel span `[p, p + 1]` with `[lo, hi]`.
///
/// @ai-generated
fn overlap(p: f64, lo: f64, hi: f64) -> f64 {
    ((p + 1.0).min(hi) - p.max(lo)).max(0.0)
}

/// The distance from `p` to the segment `[a, b]`.
///
/// @ai-generated
fn segment_distance(p: [f64; 2], a: [f64; 2], b: [f64; 2]) -> f64 {
    let ab = sub(b, a);
    let t = (dot(sub(p, a), ab) / dot(ab, ab)).clamp(0.0, 1.0);
    norm(sub(p, [a[0] + t * ab[0], a[1] + t * ab[1]]))
}

/// Call `f` on every pixel of `frame` that overlaps the box `[lo, hi]` (in pixels).
///
/// @ai-generated
fn for_pixels(
    frame: &mut RgbImage,
    lo: [f64; 2],
    hi: [f64; 2],
    mut f: impl FnMut(&mut RgbImage, u32, u32),
) {
    let span = |lo: f64, hi: f64, size: u32| {
        let first = lo.floor().max(0.0) as u32;
        let last = (hi.ceil().max(0.0) as u32).min(size);
        first..last
    };
    for y in span(lo[1], hi[1], frame.height()) {
        for x in span(lo[0], hi[0], frame.width()) {
            f(frame, x, y);
        }
    }
}

/// Blend `colour` over the pixel `(x, y)` with the opacity `alpha` in `[0, 1]`.
///
/// @ai-generated
fn blend(frame: &mut RgbImage, x: u32, y: u32, colour: Rgb<u8>, alpha: f64) {
    if alpha <= 0.0 {
        return;
    }
    let pixel = frame.get_pixel_mut(x, y);
    for (c, &target) in pixel.0.iter_mut().zip(&colour.0) {
        *c = (*c as f64 * (1.0 - alpha) + target as f64 * alpha).round() as u8;
    }
}

/// Blend `sprite` over `frame` with its alpha channel, at the offset `(x, y)`.
///
/// @ai-generated
fn add_transparent_image(frame: &mut RgbImage, sprite: &RgbaImage, x: u32, y: u32) {
    for (sx, sy, pixel) in sprite.enumerate_pixels() {
        let [r, g, b, a] = pixel.0;
        blend(frame, x + sx, y + sy, Rgb([r, g, b]), a as f64 / 255.0);
    }
}

/// Draw the outline of the cell whose top-left pixel is `(x, y)`, `thickness` pixels thick.
///
/// @ai-generated
fn draw_square_outline(frame: &mut RgbImage, x: u32, y: u32, colour: Rgb<u8>, thickness: u32) {
    for dy in 0..TILE_SIZE {
        for dx in 0..TILE_SIZE {
            let edge = dx < thickness
                || dy < thickness
                || dx >= TILE_SIZE - thickness
                || dy >= TILE_SIZE - thickness;
            if edge {
                frame.put_pixel(x + dx, y + dy, colour);
            }
        }
    }
}

#[cfg(test)]
#[path = "../unit_tests/test_continuous_rendering.rs"]
mod tests;
