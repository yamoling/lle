use std::collections::HashMap;

use super::ClauseEngine;

/// A self-filling per-time-step cache with an independent prefix for each `usize` parameter.
///
/// This supports clause families such as sequences and interdependence cycles, where generation is
/// parameterized by a trail length or cycle order in addition to the time step.
pub struct ParameterizedStepBuffer<T: Clone> {
    generate: fn(&mut ClauseEngine, usize, usize) -> Vec<T>,
    items_by_parameter: HashMap<usize, Vec<Vec<T>>>,
    capacity: usize,
}

impl<T: Clone> ParameterizedStepBuffer<T> {
    /// Create a parameterized buffer driven by `generate`.
    pub fn new(generate: fn(&mut ClauseEngine, usize, usize) -> Vec<T>, capacity: usize) -> Self {
        Self {
            generate,
            items_by_parameter: HashMap::new(),
            capacity,
        }
    }

    /// Gather items from the inclusive step range `start..=t` for one parameter.
    ///
    /// An empty iterator is returned when `start > t`; other parameter caches are unaffected.
    ///
    /// @ai-generated
    pub fn gather_range(
        &mut self,
        engine: &mut ClauseEngine,
        start: usize,
        t: usize,
        parameter: usize,
    ) -> impl Iterator<Item = T> {
        let generate = self.generate;
        let items = self
            .items_by_parameter
            .entry(parameter)
            .or_insert_with(|| Vec::with_capacity(self.capacity));

        while items.len() <= t {
            let next = items.len();
            items.push(generate(engine, next, parameter));
        }

        let end = if start > t { start } else { t + 1 };
        items[start.min(items.len())..end.min(items.len())]
            .iter()
            .flatten()
            .cloned()
    }
}

#[cfg(test)]
#[path = "../../unit_tests/test_parameterized_step_buffer.rs"]
mod tests;
