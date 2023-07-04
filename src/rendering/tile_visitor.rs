use crate::tiles::{Gem, Laser};

use super::renderer::VisitorData;

pub trait TileVisitor {
    fn visit_gem(&self, gem: &Gem, data: &mut VisitorData);
    fn visit_laser(&self, laser: &Laser, data: &mut VisitorData);
}
