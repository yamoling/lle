use std::cell::Cell;

use super::{StepBuffer, StepSource};

/// A dummy engine that counts how many times a step was generated, so tests can assert that the
/// buffer caches results instead of recomputing them.
///
/// @ai-generated
#[derive(Default)]
struct CountingEngine {
    calls: Cell<usize>,
}

/// A source that yields `[t, t + 1]` for step `t`, recording each generation on the engine.
///
/// @ai-generated
struct DoublingSource;

impl StepSource<CountingEngine> for DoublingSource {
    type Item = usize;

    fn generate_step(&self, engine: &mut CountingEngine, t: usize) -> Vec<usize> {
        engine.calls.set(engine.calls.get() + 1);
        vec![t, t + 1]
    }
}

/// `gather_until` must produce and cache every step up to `t`, generating each exactly once.
///
/// @ai-generated
#[test]
fn gather_until_generates_each_step_once() {
    let mut engine = CountingEngine::default();
    let mut buffer = StepBuffer::new(DoublingSource, 4);

    assert_eq!(
        buffer.gather_until(&mut engine, 2).collect::<Vec<_>>(),
        vec![0, 1, 1, 2, 2, 3]
    );
    assert_eq!(engine.calls.get(), 3, "steps 0, 1, 2 generated once each");
}

/// A second `gather_until` over an already-cached prefix must not regenerate anything, and
/// extending the horizon must generate only the genuinely missing steps.
///
/// @ai-generated
#[test]
fn gather_until_reuses_cached_steps() {
    let mut engine = CountingEngine::default();
    let mut buffer = StepBuffer::new(DoublingSource, 4);

    let _ = buffer.gather_until(&mut engine, 1);
    assert_eq!(engine.calls.get(), 2);

    // Re-asking for an already-cached horizon generates nothing new.
    assert_eq!(
        buffer.gather_until(&mut engine, 1).collect::<Vec<_>>(),
        vec![0, 1, 1, 2]
    );
    assert_eq!(engine.calls.get(), 2);

    // Extending the horizon only generates the missing steps (2 and 3).
    assert_eq!(
        buffer.gather_until(&mut engine, 3).collect::<Vec<_>>(),
        vec![0, 1, 1, 2, 2, 3, 3, 4]
    );
    assert_eq!(engine.calls.get(), 4);
}
