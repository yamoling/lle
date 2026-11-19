//! Construction of a discrete [`World`] from a validated map.
use crate::{
    Grid, ParseError, Position, World,
    shared::map::{MapSpec, ValidMap},
    tiles::{Gem, Laser, LaserSource, Tile, Void},
};

impl World {
    /// Build a discrete world from a validated map.
    ///
    /// This only places the tiles, wraps the beam cells into [`Laser`] tiles and places the
    /// boxes: validation and the computation of the effective starts happened in
    /// [`MapSpec::validate`].
    ///
    /// @ai-generated
    pub fn from_map(map: &ValidMap) -> Self {
        let spec = map.spec();
        let (grid, lasers_positions) = make_grid(spec);
        World::new(
            grid,
            spec.gems().clone(),
            map.starts().clone(),
            spec.voids().clone(),
            spec.exits().clone(),
            spec.walls().clone(),
            spec.boxes().clone(),
            spec.sources().iter().map(|(pos, _)| *pos).collect(),
            lasers_positions,
            spec.lifts().iter().map(|(pos, _)| *pos).collect(),
            spec.buttons().iter().map(|(pos, _)| *pos).collect(),
            spec.levers().iter().map(|(pos, _)| *pos).collect(),
            spec.pressure_plates().iter().map(|(pos, _)| *pos).collect(),
            spec.colours().clone(),
        )
    }
}

impl TryFrom<MapSpec> for World {
    type Error = ParseError;

    /// Validate `spec` and build the corresponding world.
    fn try_from(spec: MapSpec) -> Result<Self, Self::Error> {
        Ok(World::from_map(&spec.validate()?))
    }
}

/// Build the tile grid of `spec`, and return it along with the cells covered by a beam (each cell
/// once, in the order of the lasers then of the beams).
///
/// @ai-generated
fn make_grid(spec: &MapSpec) -> (Grid<Tile>, Vec<Position>) {
    let mut grid = Grid::<Tile>::new(spec.width(), spec.height(), spec.layers()).default_init();
    for pos in spec.gems() {
        grid.replace_at(pos, Tile::Gem(Gem::default()));
    }
    for pos in spec.exits() {
        grid.replace_at(pos, Tile::Exit { agent: None });
    }
    for pos in spec.voids() {
        grid.replace_at(pos, Tile::Void(Void::default()));
    }
    for pos in spec.walls() {
        grid.replace_at(pos, Tile::Wall);
    }
    for (pos, config) in spec.lifts() {
        grid.replace_at(pos, Tile::Lift(config.into()));
    }
    for (pos, config) in spec.buttons() {
        grid.replace_at(pos, Tile::Button(config.into()));
    }
    for (pos, config) in spec.levers() {
        grid.replace_at(pos, Tile::Lever(config.into()));
    }
    for (pos, config) in spec.pressure_plates() {
        grid.replace_at(pos, Tile::PressurePlate(config.into()));
    }
    let mut laser_positions = vec![];
    for (source_pos, config) in spec.sources() {
        if source_pos.k >= spec.layers() {
            continue; // Invalid layer, skip this laser
        }
        let beam = spec.beam_cells(*source_pos, config.direction);
        let source = LaserSource::from_config(config, beam.len());
        for (i, pos) in beam.into_iter().enumerate() {
            let wrapped = grid.pop(&pos);
            grid.replace_at(&pos, Tile::Laser(Laser::new(wrapped, source.beam(), i)));
            if !laser_positions.contains(&pos) {
                laser_positions.push(pos);
            }
        }
        grid.replace_at(source_pos, Tile::LaserSource(source));
    }
    (grid, laser_positions)
}
