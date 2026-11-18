use super::{geometry::*, world::ContinuousWorld};
impl ContinuousWorld {
    /// Push the disc centred on `p` out of the boundary and the solids, at most 4 times.
    ///
    /// Only the solids near `p` are looked at (see [`CellIndex`](super::cells::CellIndex)), in
    /// the order of `solids`, so the deepest one wins ties like with a scan of every solid.
    ///
    /// @ai-edited
    pub(super) fn project_static(&self, mut p: [f64; 2]) -> [f64; 2] {
        let r = self.params.agent_radius;
        for _ in 0..4 {
            p[0] = p[0].clamp(r, self.width as f64 - r);
            p[1] = p[1].clamp(r, self.height as f64 - r);
            let mut correction = [0.0; 2];
            let mut deepest = 0.0;
            for &s in self.cells.solids_near(p).as_slice() {
                let delta = sub(p, closest(p, &self.solids[s as usize]));
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
    /// Replace the content of `pairs` with the pairs `(i, j)`, `i < j`, of alive agents that
    /// overlap.
    ///
    /// @ai-edited
    fn overlapping_pairs(&self, pairs: &mut Vec<(usize, usize)>) {
        pairs.clear();
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
    }
    /// Whether the alive agent at `p` penetrates the boundary or a solid (by more than 1e-12).
    ///
    /// @ai-generated
    fn penetrates(&self, p: [f64; 2]) -> bool {
        let r = self.params.agent_radius;
        p[0] < r
            || p[1] < r
            || p[0] > self.width as f64 - r
            || p[1] > self.height as f64 - r
            || self
                .cells
                .solids_near(p)
                .as_slice()
                .iter()
                .any(|&s| distance(p, &self.solids[s as usize]) < r - 1e-12)
    }
    /// Move the agents out of the solids and of each other after their intended move from
    /// `old`, and revert to `old` the agents that still overlap something.
    ///
    /// @ai-edited
    pub(super) fn resolve_collisions(&mut self, old: &[[f64; 2]]) {
        let n = self.n_agents();
        let mut movable = std::mem::take(&mut self.scratch.movable);
        movable.clear();
        movable.extend(old.iter().enumerate().map(|(i, &p)| {
            self.state.agents_alive[i]
                && !self.state.agents_arrived[i]
                && self.state.positions[i] != p
        }));
        for (i, &can_move) in movable.iter().enumerate() {
            if can_move {
                self.state.positions[i] = self.project_static(self.state.positions[i]);
            }
        }
        let mut displacement = std::mem::take(&mut self.scratch.displacement);
        displacement.clear();
        displacement.extend(
            self.state
                .positions
                .iter()
                .zip(old)
                .map(|(&p, &q)| sub(p, q)),
        );
        let mut pairs = std::mem::take(&mut self.scratch.pairs);
        let mut corrections = std::mem::take(&mut self.scratch.corrections);
        for _ in 0..8 {
            self.overlapping_pairs(&mut pairs);
            if pairs.is_empty() {
                break;
            }
            corrections.clear();
            corrections.resize(n, [0.0; 2]);
            for &(i, j) in &pairs {
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
        let mut revert = std::mem::take(&mut self.scratch.revert);
        let mut used_fallback = false;
        loop {
            revert.clear();
            revert.extend(
                self.state
                    .positions
                    .iter()
                    .enumerate()
                    .map(|(i, &p)| self.state.agents_alive[i] && self.penetrates(p)),
            );
            self.overlapping_pairs(&mut pairs);
            for &(i, j) in &pairs {
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
        self.scratch.movable = movable;
        self.scratch.displacement = displacement;
        self.scratch.pairs = pairs;
        self.scratch.corrections = corrections;
        self.scratch.revert = revert;
    }
}
#[cfg(test)]
#[path = "../unit_tests/test_continuous_collision.rs"]
mod tests;
