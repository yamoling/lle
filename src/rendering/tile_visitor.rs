use crate::tiles::{Gem, Laser, LaserSource};

use super::renderer::VisitorData;

pub trait TileVisitor {
    fn visit_gem(&self, gem: &Gem, data: &mut VisitorData);
    fn visit_laser(&self, laser: &Laser, data: &mut VisitorData);
    fn visit_laser_source(&self, source: &LaserSource, data: &mut VisitorData);
}
