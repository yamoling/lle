//! The observations of a continuous world, as flat `f32` vectors (row-major for arrays).
//!
//! # `STATE`
//!
//! - **Agent block** (`6 + n_colours` values): `x / width`, `y / height`, `vx / max_speed`,
//!   `vy / max_speed`, `alive`, `arrived`, then the one-hot colour of the agent. The velocity is
//!   that of the state: the displacement of the last tick divided by `dt`.
//! - **Global block**: one flag per gem (1 when collected), then one flag per laser source (1
//!   when enabled), in laser order.
//!
//! The state vector is every agent block in id order, with absolute coordinates, followed by
//! the global block. The observation of an agent puts its own block first (absolute), then the
//! blocks of the other agents in increasing id order, where only `x` and `y` are relative to the
//! observer, and finally the global block.
//!
//! The optional **static block** ([`ContinuousWorld::static_features`]) describes what never
//! changes during an episode, in absolute coordinates: the centre `(x / width, y / height)` of
//! each exit, then of each gem, then of each laser source followed by its one-hot direction
//! `[N, E, S, W]`, in laser order.
//!
//! # `LAYERED`
//!
//! An array of shape `(C, height * res, width * res)`, where each cell is split into `res × res`
//! square pixels, with `C = 2 * n_colours + 4` channels in the order of LLE's `Layered`
//! observation (without its layer axis, nor its lift and actuator channels): one agent channel
//! per colour, one laser channel per colour, then `WALL`, `VOID`, `GEM` and `EXIT`. A pixel is 1
//! when its centre is:
//! - inside the disc of an alive agent of the channel's colour (dead agents are not drawn, but
//!   arrived agents are);
//! - in a wall or a laser source (as LLE, whose `wall_pos` contains the sources), in a void or
//!   in an exit;
//! - inside the pickup circle of a gem that is not collected.
//!
//! **Beams** follow another rule, since a beam (`beam_width` wide) is much thinner than a pixel
//! and would otherwise vanish whenever its axis runs between two pixel centres: a pixel of the
//! laser channel of the beam's colour is 1 when its square overlaps, with a positive area, the
//! rectangle of the current segment of an enabled beam, from the edge of the source to the cut
//! by an agent of its colour, `beam_width` wide. With `res = 1` the beam lights its cells; with
//! an odd `res` one pixel row (or column); with an even `res` the two rows on either side of its
//! axis.
//!
//! The pixels of a laser source are `-1` in the laser channel of its colour, as in LLE.
//!
//! With `res = 1`, after a reset, this is LLE's `Layered` observation of the same map: the
//! agents and the gems mark the cells that contain their centre, and the beams mark their cells.
//! The only difference is the cell of an agent standing in a beam of its own colour: the beam
//! reaches into that cell up to the agent's disc, so the cell is 1 in the laser channel, while
//! LLE turns the beam off from that cell on. The cells beyond it are 0 in both.
//!
//! # `LIDAR`
//!
//! `n_rays` rays start at the centre of the observer, evenly spaced over 360°: ray 0 points east
//! (+x) and the rays turn counter-clockwise on screen, towards the north (-y) first. Each ray is
//! cast independently against `5 + n_colours` categories, without occlusion:
//! 0. the walls and the laser sources (cells);
//! 1. the voids (cells);
//! 2. the other alive agents (discs);
//! 3. the gems that are not collected (pickup circles);
//! 4. the exits (cells);
//! 5. one category per laser colour: the current beam segments (after cuts) of the enabled
//!    sources of that colour, as rectangles `beam_width` wide.
//!
//! The value of a ray and a category is the distance to the first hit divided by `range`, or 1
//! when nothing is hit within `range` (0 when the observer's centre is inside the shape). The
//! values are ray-major (`[ray 0: category 0..C, ray 1: ...]`), followed by the observer's own
//! `STATE` block (absolute). The grid boundary is not a wall cell, so it is not hit.
use super::{ContinuousError, ContinuousWorld, geometry::*};
use crate::shared::{AgentId, CardinalDirection, Position};

/// The number of values of an agent block before its one-hot colour.
const AGENT_FEATURES: usize = 6;
/// The number of lidar categories before the laser colours.
const LIDAR_CELL_AND_DISC_CATEGORIES: usize = 5;
/// The values of a laser source in the static block: its centre and its one-hot direction.
const SOURCE_FEATURES: usize = 6;

impl ContinuousWorld {
    /// The number of values describing one agent: 6 features and a one-hot colour.
    pub fn agent_block_size(&self) -> usize {
        AGENT_FEATURES + self.n_colours()
    }

    /// The length of [`ContinuousWorld::state_vector`].
    pub fn state_size(&self) -> usize {
        self.n_agents() * self.agent_block_size() + self.gems.len() + self.beams.len()
    }

    /// The length of [`ContinuousWorld::observation`], which is that of the state vector.
    pub fn observation_size(&self) -> usize {
        self.state_size()
    }

    /// The state of the world as a flat vector: every agent block in id order, with absolute
    /// coordinates, then the global block (see the module documentation).
    ///
    /// @ai-generated
    pub fn state_vector(&self) -> Vec<f32> {
        let mut res = Vec::with_capacity(self.state_size());
        for agent in 0..self.n_agents() {
            self.push_agent_block(&mut res, agent, [0.0, 0.0]);
        }
        self.push_global_block(&mut res);
        res
    }

    /// The observation of `agent`: its own block first, with absolute coordinates, then the
    /// blocks of the other agents in increasing id order, whose `x` and `y` are relative to
    /// `agent`, then the global block (see the module documentation).
    ///
    /// `None` when `agent` is not the id of an agent of the world.
    ///
    /// @ai-generated
    pub fn observation(&self, agent: AgentId) -> Option<Vec<f32>> {
        if agent >= self.n_agents() {
            return None;
        }
        let mut res = Vec::with_capacity(self.observation_size());
        self.push_agent_block(&mut res, agent, [0.0, 0.0]);
        let origin = self.state.positions[agent];
        for other in (0..self.n_agents()).filter(|&other| other != agent) {
            self.push_agent_block(&mut res, other, origin);
        }
        self.push_global_block(&mut res);
        Some(res)
    }

    /// Append the block of `agent` to `res`, with its position relative to `origin`.
    ///
    /// @ai-generated
    fn push_agent_block(&self, res: &mut Vec<f32>, agent: AgentId, origin: [f64; 2]) {
        let [x, y] = self.state.positions[agent];
        let [vx, vy] = self.state.velocities[agent];
        let max_speed = self.params.max_speed;
        res.extend([
            ((x - origin[0]) / self.width as f64) as f32,
            ((y - origin[1]) / self.height as f64) as f32,
            (vx / max_speed) as f32,
            (vy / max_speed) as f32,
            flag(self.state.agents_alive[agent]),
            flag(self.state.agents_arrived[agent]),
        ]);
        let colour = self.colours[agent];
        res.extend((0..self.n_colours()).map(|c| flag(c == colour)));
    }

    /// Append the global block to `res`: the gem flags, then the source flags.
    fn push_global_block(&self, res: &mut Vec<f32>) {
        res.extend(self.state.gems_collected.iter().map(|&c| flag(c)));
        res.extend(self.state.sources_enabled.iter().map(|&e| flag(e)));
    }
}

impl ContinuousWorld {
    /// The length of [`ContinuousWorld::static_features`].
    pub fn static_features_size(&self) -> usize {
        2 * self.exits.len() + 2 * self.gems.len() + SOURCE_FEATURES * self.sources.len()
    }

    /// The static block (see the module documentation): the centre of each exit, then of each
    /// gem, then of each laser source with its one-hot direction `[N, E, S, W]`, in absolute
    /// coordinates.
    ///
    /// @ai-generated
    pub fn static_features(&self) -> Vec<f32> {
        let mut res = Vec::with_capacity(self.static_features_size());
        for pos in self.exits.iter().chain(&self.gems) {
            self.push_cell_centre(&mut res, pos);
        }
        for source in &self.sources {
            self.push_cell_centre(&mut res, &source.position);
            res.extend(
                [
                    CardinalDirection::North,
                    CardinalDirection::East,
                    CardinalDirection::South,
                    CardinalDirection::West,
                ]
                .map(|d| flag(d == source.direction)),
            );
        }
        res
    }

    fn push_cell_centre(&self, res: &mut Vec<f32>, pos: &Position) {
        let [x, y] = centre(pos);
        res.extend([
            (x / self.width as f64) as f32,
            (y / self.height as f64) as f32,
        ]);
    }

    /// The shape `[C, height * res, width * res]` of [`ContinuousWorld::layered`], with
    /// `C = 2 * n_colours + 4`.
    ///
    /// # Errors
    /// [`ContinuousError::InvalidParams`] when `res` is 0.
    pub fn layered_shape(&self, res: usize) -> Result<[usize; 3], ContinuousError> {
        if res == 0 {
            return Err(ContinuousError::InvalidParams {
                reason: "The resolution of the layered observation must be at least 1".into(),
            });
        }
        Ok([
            2 * self.n_colours() + 4,
            self.height * res,
            self.width * res,
        ])
    }

    /// The `LAYERED` observation with `res × res` pixels per cell, flattened in row-major order
    /// (see the module documentation and [`ContinuousWorld::layered_shape`]).
    ///
    /// # Errors
    /// [`ContinuousError::InvalidParams`] when `res` is 0.
    ///
    /// @ai-generated
    pub fn layered(&self, res: usize) -> Result<Vec<f32>, ContinuousError> {
        let shape = self.layered_shape(res)?;
        let n_colours = self.n_colours();
        let laser_0 = n_colours;
        let [wall, void, gem, exit] = std::array::from_fn(|k| 2 * n_colours + k);
        let mut layers = Layers {
            data: vec![0.0; shape.iter().product()],
            shape,
            res,
        };
        for pos in &self.solids {
            layers.fill_cell(wall, pos, 1.0);
        }
        for pos in &self.voids {
            layers.fill_cell(void, pos, 1.0);
        }
        for pos in &self.exits {
            layers.fill_cell(exit, pos, 1.0);
        }
        for (pos, &collected) in self.gems.iter().zip(&self.state.gems_collected) {
            if !collected {
                layers.fill_disc(gem, centre(pos), self.params.gem_radius);
            }
        }
        let half_width = self.params.beam_width / 2.0;
        for (beam, segment) in self.beams.iter().zip(self.beam_segments()) {
            if let Some(cut) = segment {
                layers.fill_beam(
                    laser_0 + beam.colour,
                    beam.origin,
                    beam.direction,
                    cut,
                    half_width,
                );
            }
        }
        for source in &self.sources {
            layers.fill_cell(laser_0 + source.colour, &source.position, -1.0);
        }
        for (agent, &p) in self.state.positions.iter().enumerate() {
            if self.state.agents_alive[agent] {
                layers.fill_disc(self.colours[agent], p, self.params.agent_radius);
            }
        }
        Ok(layers.data)
    }

    /// The number of lidar categories: `5 + n_colours`.
    pub fn lidar_categories(&self) -> usize {
        LIDAR_CELL_AND_DISC_CATEGORIES + self.n_colours()
    }

    /// The length of [`ContinuousWorld::lidar`]: `n_rays * (5 + n_colours)` distances, then the
    /// agent block.
    ///
    /// # Errors
    /// [`ContinuousError::InvalidParams`] when `n_rays` is 0.
    pub fn lidar_size(&self, n_rays: usize) -> Result<usize, ContinuousError> {
        if n_rays == 0 {
            return Err(ContinuousError::InvalidParams {
                reason: "The lidar needs at least one ray".into(),
            });
        }
        Ok(n_rays * self.lidar_categories() + self.agent_block_size())
    }

    /// The `LIDAR` observation of `agent`, with `n_rays` rays of length `range` (see the module
    /// documentation). Dead and arrived agents also get one, cast from their position.
    ///
    /// `Ok(None)` when `agent` is not the id of an agent of the world.
    ///
    /// # Errors
    /// [`ContinuousError::InvalidParams`] when `n_rays` is 0 or `range` is not strictly positive
    /// and finite.
    ///
    /// @ai-generated
    pub fn lidar(
        &self,
        agent: AgentId,
        n_rays: usize,
        range: f64,
    ) -> Result<Option<Vec<f32>>, ContinuousError> {
        let size = self.lidar_size(n_rays)?;
        if !range.is_finite() || range <= 0.0 {
            return Err(ContinuousError::InvalidParams {
                reason: format!("The range of the lidar must be positive and finite, got {range}"),
            });
        }
        if agent >= self.n_agents() {
            return Ok(None);
        }
        let origin = self.state.positions[agent];
        let shapes = self.lidar_shapes(agent);
        let mut res = Vec::with_capacity(size);
        for ray in 0..n_rays {
            let direction = ray_direction(ray, n_rays);
            for category in &shapes {
                let hit = category
                    .iter()
                    .filter_map(|shape| shape.hit(origin, direction))
                    .fold(f64::INFINITY, f64::min);
                res.push((hit.min(range) / range) as f32);
            }
        }
        self.push_agent_block(&mut res, agent, [0.0, 0.0]);
        Ok(Some(res))
    }

    /// The shapes of every lidar category, as seen by `agent` (which does not see itself).
    ///
    /// @ai-generated
    fn lidar_shapes(&self, agent: AgentId) -> Vec<Vec<Shape>> {
        let cells = |cells: &[Position]| cells.iter().map(Shape::cell).collect::<Vec<_>>();
        let r = self.params.agent_radius;
        let others = (0..self.n_agents())
            .filter(|&other| other != agent && self.state.agents_alive[other])
            .map(|other| Shape::Disc(self.state.positions[other], r))
            .collect();
        let gems = self
            .gems
            .iter()
            .zip(&self.state.gems_collected)
            .filter(|&(_, &collected)| !collected)
            .map(|(pos, _)| Shape::Disc(centre(pos), self.params.gem_radius))
            .collect();
        let mut shapes = vec![
            cells(&self.solids),
            cells(&self.voids),
            others,
            gems,
            cells(&self.exits),
        ];
        shapes.extend((0..self.n_colours()).map(|_| Vec::new()));
        let half_width = self.params.beam_width / 2.0;
        for (beam, segment) in self.beams.iter().zip(self.beam_segments()) {
            let Some(cut) = segment else { continue };
            let (lo, hi) = beam_rectangle(beam.origin, beam.direction, cut, half_width);
            shapes[LIDAR_CELL_AND_DISC_CATEGORIES + beam.colour].push(Shape::Box(lo, hi));
        }
        shapes
    }
}

/// The corners `(lo, hi)` of the rectangle of a beam from `origin` to `cut` along the axis
/// `direction`, `half_width` on each side of its axis.
///
/// @ai-generated
pub(super) fn beam_rectangle(
    origin: [f64; 2],
    direction: [f64; 2],
    cut: f64,
    half_width: f64,
) -> ([f64; 2], [f64; 2]) {
    let end = [
        origin[0] + direction[0] * cut,
        origin[1] + direction[1] * cut,
    ];
    // Beams are axis-aligned: widen the segment across its direction only.
    let across = [direction[1].abs(), direction[0].abs()];
    let lo = std::array::from_fn(|d| origin[d].min(end[d]) - across[d] * half_width);
    let hi = std::array::from_fn(|d| origin[d].max(end[d]) + across[d] * half_width);
    (lo, hi)
}

/// The unit direction of the ray `ray` out of `n_rays`, in world coordinates: ray 0 points east
/// and the rays turn counter-clockwise on screen (towards -y first). The components of the rays
/// along the axes are exact.
///
/// @ai-generated
fn ray_direction(ray: usize, n_rays: usize) -> [f64; 2] {
    let angle = std::f64::consts::TAU * ray as f64 / n_rays as f64;
    let snap = |v: f64| if v.abs() < 1e-12 { 0.0 } else { v };
    [snap(angle.cos()), snap(-angle.sin())]
}

/// A shape hit by the lidar.
enum Shape {
    /// An axis-aligned box `[lo, hi]`.
    Box([f64; 2], [f64; 2]),
    /// A disc of the given centre and radius.
    Disc([f64; 2], f64),
}

impl Shape {
    fn cell(pos: &Position) -> Self {
        let (x, y) = (pos.j as f64, pos.i as f64);
        Shape::Box([x, y], [x + 1.0, y + 1.0])
    }

    fn hit(&self, origin: [f64; 2], direction: [f64; 2]) -> Option<f64> {
        match *self {
            Shape::Box(lo, hi) => ray_box(origin, direction, lo, hi),
            Shape::Disc(centre, radius) => ray_disc(origin, direction, centre, radius),
        }
    }
}

/// The channels of a layered observation being filled, of shape `[C, height * res, width * res]`.
struct Layers {
    data: Vec<f32>,
    shape: [usize; 3],
    res: usize,
}

impl Layers {
    fn set(&mut self, channel: usize, row: usize, col: usize, value: f32) {
        let [_, height, width] = self.shape;
        self.data[(channel * height + row) * width + col] = value;
    }

    /// The pixel centre `(row, col)`, in world units.
    fn pixel_centre(&self, row: usize, col: usize) -> [f64; 2] {
        let res = self.res as f64;
        [(col as f64 + 0.5) / res, (row as f64 + 0.5) / res]
    }

    /// The pixels whose centre may lie in `[lo, hi]` (world units), as `(rows, cols)` ranges.
    fn pixel_ranges(
        &self,
        lo: [f64; 2],
        hi: [f64; 2],
    ) -> (std::ops::Range<usize>, std::ops::Range<usize>) {
        let res = self.res as f64;
        let [_, height, width] = self.shape;
        let span = |lo: f64, hi: f64, size: usize| {
            let first = (lo * res - 0.5).floor().max(0.0) as usize;
            let last = ((hi * res + 0.5).ceil().max(0.0) as usize).min(size);
            first..last
        };
        (span(lo[1], hi[1], height), span(lo[0], hi[0], width))
    }

    fn fill_cell(&mut self, channel: usize, pos: &Position, value: f32) {
        for row in pos.i * self.res..(pos.i + 1) * self.res {
            for col in pos.j * self.res..(pos.j + 1) * self.res {
                self.set(channel, row, col, value);
            }
        }
    }

    /// Set to 1 the pixels whose centre is strictly inside the disc.
    fn fill_disc(&mut self, channel: usize, centre: [f64; 2], radius: f64) {
        let lo = [centre[0] - radius, centre[1] - radius];
        let hi = [centre[0] + radius, centre[1] + radius];
        let (rows, cols) = self.pixel_ranges(lo, hi);
        for row in rows {
            for col in cols.clone() {
                if norm(sub(self.pixel_centre(row, col), centre)) < radius {
                    self.set(channel, row, col, 1.0);
                }
            }
        }
    }

    /// Set to 1 the pixels whose square overlaps, with a positive area, the rectangle of the beam
    /// from its origin to `cut`, `half_width` on each side of its axis. Beams are axis-aligned.
    ///
    /// @ai-generated
    fn fill_beam(
        &mut self,
        channel: usize,
        origin: [f64; 2],
        direction: [f64; 2],
        cut: f64,
        half_width: f64,
    ) {
        let (lo, hi) = beam_rectangle(origin, direction, cut, half_width);
        let res = self.res as f64;
        // The pixel `k` spans `[k / res, (k + 1) / res]`, exactly at integer coordinates.
        let overlaps = |k: usize, lo: f64, hi: f64| {
            ((k + 1) as f64 / res).min(hi) - (k as f64 / res).max(lo) > 0.0
        };
        let (rows, cols) = self.pixel_ranges(lo, hi);
        for row in rows {
            for col in cols.clone() {
                if overlaps(col, lo[0], hi[0]) && overlaps(row, lo[1], hi[1]) {
                    self.set(channel, row, col, 1.0);
                }
            }
        }
    }
}

fn flag(value: bool) -> f32 {
    if value { 1.0 } else { 0.0 }
}

#[cfg(test)]
#[path = "../unit_tests/test_continuous_observations.rs"]
mod tests;
