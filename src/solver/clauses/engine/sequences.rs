use crate::solver::{Clause, VarKey, clauses::ClauseEngine};

impl ClauseEngine {
    /// Generate a sparse Horn encoding that forbids temporal help trails of exactly `length`.
    ///
    /// A progress state means that one static sequence-pattern prefix is temporally realizable by `t`.
    /// Distinct arcs may occur simultaneously, while a repeated static arc additionally requires its
    /// previous occurrence by `t - 1`, preventing reuse of the same temporal help edge. States and
    /// transitions are materialized only when their help literals and predecessor prefixes exist.
    pub fn generate_sequence_clauses(&mut self, t: usize, length: usize) -> Vec<Clause> {
        self.ctx.update(t);
        let n_helpers = self
            .ctx
            .laser_sources
            .iter()
            .map(|source| source.agent_id)
            .collect::<std::collections::HashSet<_>>()
            .len();
        let max_temporal_edges = n_helpers
            .saturating_mul(self.ctx.n_agents.saturating_sub(1))
            .saturating_mul(self.ctx.t_max + 1);
        if length == 0 || length > max_temporal_edges {
            return Vec::new();
        }

        let patterns = self.sequence_patterns(length);
        let mut clauses = Vec::new();

        for (pattern_index, pattern) in patterns.iter().enumerate() {
            for (arc_index, arc) in pattern.arcs.iter().enumerate() {
                let prefix_len = arc_index + 1;
                let is_complete = prefix_len == length;
                let previous_progress = if prefix_len > 1 {
                    self.pool.get(&VarKey::SequenceProgress {
                        length,
                        pattern: pattern_index,
                        prefix_len: prefix_len - 1,
                        t,
                    })
                } else {
                    None
                };
                if prefix_len > 1 && previous_progress.is_none() {
                    continue;
                }

                let repeated_arc_progress = if let Some(previous_prefix_len) =
                    pattern.previous_same_prefix_len[arc_index]
                {
                    if t == 0 {
                        continue;
                    }
                    self.pool.get(&VarKey::SequenceProgress {
                        length,
                        pattern: pattern_index,
                        prefix_len: previous_prefix_len,
                        t: t - 1,
                    })
                } else {
                    None
                };
                if pattern.previous_same_prefix_len[arc_index].is_some()
                    && repeated_arc_progress.is_none()
                {
                    continue;
                }

                let help = self.pool.get(&VarKey::Help {
                    helper: arc.helper,
                    beneficiary: arc.beneficiary,
                    t,
                });
                let previous_time = (t > 0).then(|| {
                    self.pool.get(&VarKey::SequenceProgress {
                        length,
                        pattern: pattern_index,
                        prefix_len,
                        t: t - 1,
                    })
                });
                if help.is_none() && previous_time.flatten().is_none() {
                    continue;
                }

                if !is_complete {
                    let progress = self
                        .pool
                        .sequence_progress(length, pattern_index, prefix_len, t);
                    if let Some(previous_time) = previous_time.flatten() {
                        clauses.push(vec![-previous_time, progress]);
                    }
                    if let Some(help) = help {
                        let mut transition = vec![-help];
                        if let Some(previous_progress) = previous_progress {
                            transition.push(-previous_progress);
                        }
                        if let Some(repeated_arc_progress) = repeated_arc_progress {
                            transition.push(-repeated_arc_progress);
                        }
                        transition.push(progress);
                        clauses.push(transition);
                    }
                } else if let Some(help) = help {
                    let mut blocking = vec![-help];
                    if let Some(previous_progress) = previous_progress {
                        blocking.push(-previous_progress);
                    }
                    if let Some(repeated_arc_progress) = repeated_arc_progress {
                        blocking.push(-repeated_arc_progress);
                    }
                    clauses.push(blocking);
                }
            }
        }
        clauses
    }
}

#[cfg(test)]
#[path = "../../../unit_tests/engine/test_sequences.rs"]
mod tests;
