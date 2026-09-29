use crate::{World, solver::clauses::engine::ClauseEngine};

use super::ParameterizedStepBuffer;

fn tiny_engine() -> ClauseEngine {
    let world = World::try_from("S0 X").expect("failed to build test world");
    ClauseEngine::new(&world, 3)
}

fn generate_marker(engine: &mut ClauseEngine, _t: usize, _parameter: usize) -> Vec<i32> {
    vec![engine.pool.aux()]
}

/// Each parameter has an independent cache, and cached steps are generated only once.
///
/// @ai-generated
#[test]
fn parameters_cache_independent_prefixes() {
    let mut engine = tiny_engine();
    let mut buffer = ParameterizedStepBuffer::new(generate_marker, 4);

    let first = buffer
        .gather_range(&mut engine, 0, 1, 2)
        .collect::<Vec<_>>();
    assert_eq!(engine.n_vars(), 2);

    let repeated = buffer
        .gather_range(&mut engine, 0, 1, 2)
        .collect::<Vec<_>>();
    assert_eq!(repeated, first);
    assert_eq!(engine.n_vars(), 2);

    let other_parameter = buffer
        .gather_range(&mut engine, 0, 0, 3)
        .collect::<Vec<_>>();
    assert_eq!(other_parameter.len(), 1);
    assert_eq!(engine.n_vars(), 3);

    let extended = buffer
        .gather_range(&mut engine, 0, 2, 2)
        .collect::<Vec<_>>();
    assert_eq!(extended.len(), 3);
    assert_eq!(engine.n_vars(), 4);
}

/// Parameterized range gathering returns the selected parameter's cached suffix only.
///
/// @ai-generated
#[test]
fn parameterized_range_reads_cached_suffix_without_crossing_parameters() {
    let mut engine = tiny_engine();
    let mut buffer = ParameterizedStepBuffer::new(generate_marker, 4);
    let full = buffer
        .gather_range(&mut engine, 0, 2, 2)
        .collect::<Vec<_>>();
    let suffix = buffer
        .gather_range(&mut engine, 1, 2, 2)
        .collect::<Vec<_>>();
    let other = buffer
        .gather_range(&mut engine, 1, 2, 3)
        .collect::<Vec<_>>();

    assert_eq!(suffix, full[1..]);
    assert_eq!(other.len(), 2);
    assert!(suffix.iter().all(|literal| !other.contains(literal)));
}
