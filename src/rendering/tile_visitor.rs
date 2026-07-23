use crate::tiles::{Button, Gem, Laser, LaserSource, Lift};

use super::renderer::VisitorData;

pub trait TileVisitor {
    fn visit_gem(&self, gem: &Gem, data: &mut VisitorData);
    fn visit_laser(&self, laser: &Laser, data: &mut VisitorData);
    fn visit_laser_source(&self, source: &LaserSource, data: &mut VisitorData);
    fn visit_lift(&self, lift: &Lift, data: &mut VisitorData);
    fn visit_button(&self, button: &Button, data: &mut VisitorData);
}
