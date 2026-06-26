use super::ClauseEngine;

/// A self-filling, per-time-step cache.
///
/// `StepBuffer` is the single home of the "fill lazily up to `t`, then gather steps `0..=t`"
/// pattern. Callers ask for [`gather_until`](Self::gather_until); the buffer transparently calls its
/// generation function for any step it has not produced yet, caches the result, and returns the
/// whole prefix. Nothing outside the buffer needs to track how far generation has progressed.
pub struct StepBuffer<T: Clone> {
    generate: fn(&mut ClauseEngine, usize) -> Vec<T>,
    items: Vec<Vec<T>>,
}

impl<T: Clone> StepBuffer<T> {
    /// Create a buffer driven by `generate`, reserving room for `capacity` time steps.
    pub fn new(generate: fn(&mut ClauseEngine, usize) -> Vec<T>, capacity: usize) -> Self {
        Self {
            generate,
            items: Vec::with_capacity(capacity),
        }
    }

    /// Gather every item from step `0` through step `t`, generating and caching any step not yet
    /// produced by calling the generation function for each missing step in order.
    pub fn gather_until(&mut self, engine: &mut ClauseEngine, t: usize) -> impl Iterator<Item = T> {
        while self.items.len() <= t {
            let next = self.items.len();
            let step = (self.generate)(engine, next);
            self.items.push(step);
        }
        self.items[..=t].iter().flatten().cloned()
    }
}

#[cfg(test)]
#[path = "../../unit_tests/test_step_buffer.rs"]
mod tests;
