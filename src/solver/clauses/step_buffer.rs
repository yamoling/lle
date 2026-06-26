use std::marker::PhantomData;

/// A rule that produces the items for one time step, given mutable access to an engine `E`.
///
/// Implementors own whatever auxiliary data a clause family needs (e.g. the enumerated chains of a
/// given length) and delegate the actual work to the engine. Because `generate_step` takes the
/// source by shared reference and the engine by exclusive reference, a [`StepBuffer`] can call it
/// while it is itself a field of a larger struct that also owns the engine — the two borrows are
/// disjoint.
pub trait StepSource<E> {
    type Item: Clone;

    /// Produce every item belonging to step `t`.
    fn generate_step(&self, engine: &mut E, t: usize) -> Vec<Self::Item>;
}

/// A self-filling, per-time-step cache.
///
/// `StepBuffer` is the single home of the "fill lazily up to `t`, then gather steps `0..=t`"
/// pattern. Callers ask for [`gather_until`](Self::gather_until); the buffer transparently calls its
/// [`StepSource`] for any step it has not produced yet, caches the result, and returns the whole
/// prefix. Nothing outside the buffer needs to track how far generation has progressed.
pub struct StepBuffer<E, S: StepSource<E>> {
    source: S,
    items: Vec<Vec<S::Item>>,
    _engine: PhantomData<fn(&mut E)>,
}

impl<E, S: StepSource<E>> StepBuffer<E, S> {
    /// Create a buffer driven by `source`, reserving room for `capacity` time steps.
    pub fn new(source: S, capacity: usize) -> Self {
        Self {
            source,
            items: Vec::with_capacity(capacity),
            _engine: PhantomData,
        }
    }

    /// The generation rule backing this buffer. Lets the façade inspect auxiliary source data
    /// (e.g. how many chains were enumerated) without duplicating it.
    pub fn source(&self) -> &S {
        &self.source
    }

    /// Gather every item from step `0` through step `t`, generating and caching any step not yet
    /// produced by calling the [`StepSource`] for each missing step in order.
    pub fn gather_until(&mut self, engine: &mut E, t: usize) -> impl Iterator<Item = S::Item> {
        while self.items.len() <= t {
            let next = self.items.len();
            let step = self.source.generate_step(engine, next);
            self.items.push(step);
        }
        self.items[..=t].iter().flatten().cloned()
    }
}

#[cfg(test)]
#[path = "../../unit_tests/test_step_buffer.rs"]
mod tests;
