use crate::solver::{Clause, VarKey, clauses::ClauseEngine};

impl ClauseEngine {
    /// Generate exact-order closed-trail rejection clauses for one time column.
    ///
    /// @ai-generated
    ///
    /// The private auxiliary states encode greedy timestamp realizability for every proper prefix of
    /// each static, deletion-irreducible rooted pattern. The final transition is emitted directly as a
    /// blocking clause instead of allocating a progress state that would immediately be forced false.
    /// This one-sided (at-most-one) Horn encoding is intentionally suitable only for
    /// `NoInterdependence`: a completed reachable prefix is forbidden at every horizon.
    pub fn generate_interdependence_clauses(&mut self, t: usize, order: usize) -> Vec<Clause> {
        self.ctx.update(t);
        let patterns = self.interdependence_patterns(order);
        let mut clauses = Vec::new();

        for (pattern_index, pattern) in patterns.iter().enumerate() {
            let pattern_len = pattern.arcs.len();
            for (arc_index, arc) in pattern.arcs.iter().enumerate() {
                let prefix_len = arc_index + 1;
                let is_complete = prefix_len == pattern_len;
                if !is_complete {
                    let progress =
                        self.pool
                            .interdependence_progress(order, pattern_index, prefix_len, t);
                    if t > 0 {
                        let previous_time = self.pool.interdependence_progress(
                            order,
                            pattern_index,
                            prefix_len,
                            t - 1,
                        );
                        clauses.push(vec![-previous_time, progress]);
                    }
                }

                let Some(help) = self.pool.get(&VarKey::Help {
                    helper: arc.helper,
                    beneficiary: arc.beneficiary,
                    t,
                }) else {
                    continue;
                };
                let mut transition = vec![-help];
                if prefix_len > 1 {
                    transition.push(-self.pool.interdependence_progress(
                        order,
                        pattern_index,
                        prefix_len - 1,
                        t,
                    ));
                }
                if let Some(previous_prefix_len) = pattern.previous_same_prefix_len[arc_index] {
                    if t == 0 {
                        continue;
                    }
                    transition.push(-self.pool.interdependence_progress(
                        order,
                        pattern_index,
                        previous_prefix_len,
                        t - 1,
                    ));
                }
                if !is_complete {
                    transition.push(self.pool.interdependence_progress(
                        order,
                        pattern_index,
                        prefix_len,
                        t,
                    ));
                }
                clauses.push(transition);
            }
        }
        clauses
    }
}

#[cfg(test)]
#[path = "../../../unit_tests/engine/test_cycles.rs"]
mod tests;
