use std::collections::HashMap;

use super::ClauseEngine;

/// A self-filling per-time-step cache with an independent prefix for each `usize` parameter.
///
/// This supports clause families such as chains and interdependence cycles, where generation is
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

    /// Gather every item from step `0` through `t` for `parameter`, generating missing steps.
    ///
    /// Each parameter owns an independent incremental cache, so extending one parameter does not
    /// populate or invalidate any other parameter's prefix.
    pub fn gather_until(
        &mut self,
        engine: &mut ClauseEngine,
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

        items[..=t].iter().flatten().cloned()
    }
}

#[cfg(test)]
#[path = "../../unit_tests/test_parameterized_step_buffer.rs"]
mod tests;
