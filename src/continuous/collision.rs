use super::{geometry::*, world::ContinuousWorld};
impl ContinuousWorld {
    pub(super) fn project_static(&self, mut p: [f64; 2]) -> [f64; 2] {
        let r = self.params.agent_radius;
        for _ in 0..4 {
            p[0] = p[0].clamp(r, self.width as f64 - r);
            p[1] = p[1].clamp(r, self.height as f64 - r);
            let mut correction = [0.0; 2];
            let mut deepest = 0.0;
            for cell in &self.solids {
                let delta = sub(p, closest(p, cell));
                let d = norm(delta);
                if d < r && r - d > deepest && d > 0.0 {
                    deepest = r - d;
                    correction = [delta[0] / d * deepest, delta[1] / d * deepest];
                }
            }
            if deepest == 0.0 {
                break;
            }
            p[0] += correction[0];
            p[1] += correction[1];
        }
        p
    }
    pub(super) fn overlapping_pairs(&self) -> Vec<(usize, usize)> {
        let mut pairs = vec![];
        for i in 0..self.n_agents() {
            if !self.state.agents_alive[i] {
                continue;
            }
            for j in i + 1..self.n_agents() {
                if self.state.agents_alive[j]
                    && norm(sub(self.state.positions[j], self.state.positions[i]))
                        < 2.0 * self.params.agent_radius - 1e-12
                {
                    pairs.push((i, j));
                }
            }
        }
        pairs
    }
    pub(super) fn resolve_collisions(&mut self, old: &[[f64; 2]]) {
        let movable: Vec<_> = old
            .iter()
            .enumerate()
            .map(|(i, &p)| {
                self.state.agents_alive[i]
                    && !self.state.agents_arrived[i]
                    && self.state.positions[i] != p
            })
            .collect();
        for (i, &can_move) in movable.iter().enumerate() {
            if can_move {
                self.state.positions[i] = self.project_static(self.state.positions[i]);
            }
        }
        let displacement: Vec<_> = self
            .state
            .positions
            .iter()
            .zip(old)
            .map(|(&p, &q)| sub(p, q))
            .collect();
        for _ in 0..8 {
            let pairs = self.overlapping_pairs();
            if pairs.is_empty() {
                break;
            }
            let mut corrections = vec![[0.0; 2]; self.n_agents()];
            for (i, j) in pairs {
                let delta = sub(self.state.positions[j], self.state.positions[i]);
                let d = norm(delta);
                let n = if d == 0.0 {
                    [1.0, 0.0]
                } else {
                    [delta[0] / d, delta[1] / d]
                };
                let wi = if !movable[i] {
                    0.0
                } else {
                    dot(displacement[i], n).max(0.0)
                };
                let wj = if !movable[j] {
                    0.0
                } else {
                    (-dot(displacement[j], n)).max(0.0)
                };
                let share = if wi + wj > 0.0 {
                    wi / (wi + wj)
                } else if !movable[i] {
                    0.0
                } else if !movable[j] {
                    1.0
                } else {
                    0.5
                };
                let penetration = 2.0 * self.params.agent_radius - d;
                for (axis, &normal) in n.iter().enumerate() {
                    corrections[i][axis] -= normal * penetration * share;
                    corrections[j][axis] += normal * penetration * (1.0 - share);
                }
            }
            for (i, c) in corrections.iter().enumerate() {
                if movable[i] {
                    let p = self.state.positions[i];
                    self.state.positions[i] = self.project_static([p[0] + c[0], p[1] + c[1]]);
                }
            }
        }
        let mut used_fallback = false;
        loop {
            let r = self.params.agent_radius;
            let mut revert: Vec<_> = self
                .state
                .positions
                .iter()
                .enumerate()
                .map(|(i, &p)| {
                    self.state.agents_alive[i]
                        && (p[0] < r
                            || p[1] < r
                            || p[0] > self.width as f64 - r
                            || p[1] > self.height as f64 - r
                            || self.solids.iter().any(|cell| distance(p, cell) < r - 1e-12))
                })
                .collect();
            for (i, j) in self.overlapping_pairs() {
                revert[i] = true;
                revert[j] = true;
            }
            if !revert.iter().any(|&r| r) {
                break;
            }
            used_fallback = true;
            for (i, &revert) in revert.iter().enumerate() {
                if revert {
                    self.state.positions[i] = old[i];
                }
            }
        }
        if used_fallback {
            self.collision_fallbacks += 1;
        }
    }
}
#[cfg(test)]
#[path = "../unit_tests/test_continuous_collision.rs"]
mod tests;
