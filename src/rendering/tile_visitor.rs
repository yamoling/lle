use crate::tiles::{Gem, Laser, LaserSource, Start};

pub trait TileVisitor {
    fn visit_wall(&mut self, x: u32, y: u32);
    fn visit_gem(&mut self, gem: &Gem, x: u32, y: u32);
    fn visit_start(&mut self, start: &Start, x: u32, y: u32);
    fn visit_exit(&mut self, x: u32, y: u32);
    fn visit_laser_source(&mut self, laser_source: &LaserSource, x: u32, y: u32);
    fn visit_laser(&mut self, laser: &Laser, x: u32, y: u32);
}
