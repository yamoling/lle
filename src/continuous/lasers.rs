use super::{geometry::*, world::ContinuousWorld};
use crate::{Position, WorldEvent};
#[derive(Debug)]
pub(super) struct Beam {
    origin: [f64; 2],
    direction: [f64; 2],
    length: f64,
    colour: usize,
}
impl Beam {
    pub(super) fn new(
        p: Position,
        delta: (i32, i32),
        colour: usize,
        width: usize,
        height: usize,
        solids: &[Position],
    ) -> Self {
        let direction = [delta.1 as f64, delta.0 as f64];
        let c = centre(&p);
        let origin = [c[0] + direction[0] * 0.5, c[1] + direction[1] * 0.5];
        let (mut i, mut j) = (p.i as i32 + delta.0, p.j as i32 + delta.1);
        let mut length = 0.0;
        while i >= 0 && j >= 0 && i < height as i32 && j < width as i32 {
            if solids
                .iter()
                .any(|s| s.i == i as usize && s.j == j as usize)
            {
                break;
            }
            length += 1.0;
            i += delta.0;
            j += delta.1;
        }
        Self {
            origin,
            direction,
            length,
            colour,
        }
    }
    fn contact(&self, p: [f64; 2], r: f64, half_width: f64) -> Option<f64> {
        let delta = sub(p, self.origin);
        let s = dot(delta, self.direction);
        let h = (delta[0] * self.direction[1] - delta[1] * self.direction[0]).abs();
        if h >= r + half_width {
            return None;
        }
        let extent = (r * r - (h - half_width).max(0.0).powi(2)).sqrt();
        if s + extent <= 0.0 || s - extent >= self.length {
            None
        } else {
            Some(s - extent)
        }
    }
}
impl ContinuousWorld {
    pub(super) fn resolve_lasers(&mut self, events: &mut Vec<WorldEvent>) {
        loop {
            let mut deaths = vec![false; self.n_agents()];
            for (b, beam) in self.beams.iter().enumerate() {
                if !self.state.sources_enabled[b] {
                    continue;
                }
                let contacts: Vec<_> = self
                    .state
                    .positions
                    .iter()
                    .map(|&p| {
                        beam.contact(p, self.params.agent_radius, self.params.beam_width / 2.0)
                    })
                    .collect();
                let mut cut = beam.length;
                for (i, contact) in contacts.iter().enumerate() {
                    if self.state.agents_alive[i]
                        && self.colours[i] == beam.colour
                        && let Some(s) = contact
                    {
                        cut = cut.min(*s);
                    }
                }
                for (i, contact) in contacts.iter().enumerate() {
                    if self.state.agents_alive[i]
                        && self.colours[i] != beam.colour
                        && contact.is_some_and(|s| s < cut)
                    {
                        deaths[i] = true;
                    }
                }
            }
            if !deaths.iter().any(|&d| d) {
                break;
            }
            for (i, died) in deaths.into_iter().enumerate() {
                if died {
                    self.kill(i, events);
                }
            }
        }
    }
}
#[cfg(test)]
#[path = "../unit_tests/test_continuous_lasers.rs"]
mod tests;
