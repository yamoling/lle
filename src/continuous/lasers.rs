use super::{geometry::*, world::ContinuousWorld};
use crate::shared::{Position, WorldEvent};
/// The beam of a laser source: it starts on the edge of the source cell, at `origin`, and runs
/// along `direction` (a unit axis vector) for `length` cells, up to the first solid cell or the
/// boundary.
#[derive(Clone, Debug)]
pub(super) struct Beam {
    pub(super) origin: [f64; 2],
    pub(super) direction: [f64; 2],
    pub(super) length: f64,
    pub(super) colour: usize,
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
    /// The contact abscissa of every agent with `beam` (see [`Beam::contact`]), indexed by agent
    /// id, dead agents included.
    fn beam_contacts(&self, beam: &Beam) -> Vec<Option<f64>> {
        self.state
            .positions
            .iter()
            .map(|&p| beam.contact(p, self.params.agent_radius, self.params.beam_width / 2.0))
            .collect()
    }

    /// Where `beam` is cut, as a distance from its origin: the first contact of an alive agent
    /// of its colour, or its full length. It can be negative when that agent overlaps the
    /// origin.
    ///
    /// @ai-generated
    fn beam_cut(&self, beam: &Beam, contacts: &[Option<f64>]) -> f64 {
        let mut cut = beam.length;
        for (i, contact) in contacts.iter().enumerate() {
            if self.state.agents_alive[i]
                && self.colours[i] == beam.colour
                && let Some(s) = contact
            {
                cut = cut.min(*s);
            }
        }
        cut
    }

    /// The current segment of each beam, in laser order, as the length of the beam from its
    /// origin after the cut by an alive agent of its colour. `None` when the source is disabled
    /// or the segment is empty (the cutting agent overlaps the origin).
    ///
    /// @ai-generated
    pub(super) fn beam_segments(&self) -> Vec<Option<f64>> {
        self.beams
            .iter()
            .zip(&self.state.sources_enabled)
            .map(|(beam, &enabled)| {
                if !enabled {
                    return None;
                }
                let cut = self.beam_cut(beam, &self.beam_contacts(beam));
                (cut > 0.0).then_some(cut)
            })
            .collect()
    }

    pub(super) fn resolve_lasers(&mut self, events: &mut Vec<WorldEvent>) {
        loop {
            let mut deaths = vec![false; self.n_agents()];
            for (b, beam) in self.beams.iter().enumerate() {
                if !self.state.sources_enabled[b] {
                    continue;
                }
                let contacts = self.beam_contacts(beam);
                let cut = self.beam_cut(beam, &contacts);
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
