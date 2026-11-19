//! Tests of `Action::between`, which replaces `impl Sub<Position> for Position`: `to - from`
//! became `Action::between(from, to)`.
use super::Action;
use crate::{Position, RuntimeWorldError};

fn pos(i: usize, j: usize, k: usize) -> Position {
    Position { i, j, k }
}

#[test]
fn between_a_cell_and_itself_is_stay() {
    for p in [pos(0, 0, 0), pos(3, 2, 0), pos(1, 4, 2)] {
        assert_eq!(Action::between(p, p).unwrap(), Action::Stay, "{p:?}");
    }
}

#[test]
fn between_neighbours_gives_the_four_directions_on_any_layer() {
    // North decreases the row index i, East increases the column index j.
    for k in [0, 2] {
        let from = pos(2, 2, k);
        let cases = [
            (pos(1, 2, k), Action::North),
            (pos(3, 2, k), Action::South),
            (pos(2, 3, k), Action::East),
            (pos(2, 1, k), Action::West),
        ];
        for (to, expected) in cases {
            assert_eq!(Action::between(from, to).unwrap(), expected, "{to:?}");
        }
    }
}

#[test]
fn between_non_adjacent_cells_is_an_invalid_delta() {
    // The delta is `to - from`, as with the former `to - from` on positions.
    let from = pos(2, 2, 0);
    let cases = [
        (pos(0, 2, 0), (-2, 0)),
        (pos(2, 5, 0), (0, 3)),
        (pos(3, 3, 0), (1, 1)),
        (pos(1, 1, 0), (-1, -1)),
    ];
    for (to, (expected_di, expected_dj)) in cases {
        match Action::between(from, to) {
            Err(RuntimeWorldError::InvalidActionDelta { di, dj }) => {
                assert_eq!((di, dj), (expected_di, expected_dj), "{to:?}");
            }
            other => panic!("{to:?}: expected InvalidActionDelta, got {other:?}"),
        }
    }
}

#[test]
fn between_cells_on_different_layers_is_not_adjacent() {
    // Same (i, j) on another layer, and a planar neighbour on another layer.
    let from = pos(1, 1, 0);
    for to in [pos(1, 1, 1), pos(0, 1, 1), pos(1, 2, 2)] {
        for (a, b) in [(from, to), (to, from)] {
            match Action::between(a, b) {
                Err(RuntimeWorldError::PositionsNotAdjacent { pos_0, pos1 }) => {
                    assert_eq!((pos_0, pos1), (a, b));
                }
                other => panic!("{a:?} -> {b:?}: expected PositionsNotAdjacent, got {other:?}"),
            }
        }
    }
}

#[test]
fn between_large_coordinates_neither_aliases_nor_overflows() {
    let max = usize::MAX;
    // Neighbours at the edges of the `usize` range.
    assert_eq!(
        Action::between(pos(max, 0, 0), pos(max - 1, 0, 0)).unwrap(),
        Action::North
    );
    assert_eq!(
        Action::between(pos(max - 1, 0, 0), pos(max, 0, 0)).unwrap(),
        Action::South
    );
    assert_eq!(
        Action::between(pos(0, max - 1, 0), pos(0, max, 0)).unwrap(),
        Action::East
    );
    assert_eq!(
        Action::between(pos(0, max, 0), pos(0, max - 1, 0)).unwrap(),
        Action::West
    );
    // Neighbours around `i32::MAX`, which a cast to `i32` would wrap.
    let edge = i32::MAX as usize;
    assert_eq!(
        Action::between(pos(edge, 0, 0), pos(edge + 1, 0, 0)).unwrap(),
        Action::South
    );
    assert_eq!(
        Action::between(pos(0, edge + 1, 0), pos(0, edge, 0)).unwrap(),
        Action::West
    );
    // A large but representable delta keeps its value.
    match Action::between(pos(0, 0, 0), pos(edge, 0, 0)) {
        Err(RuntimeWorldError::InvalidActionDelta { di, dj }) => {
            assert_eq!((di, dj), (i32::MAX, 0))
        }
        other => panic!("expected InvalidActionDelta, got {other:?}"),
    }
    // Deltas that do not fit in an `i32`, including one that a 32-bit truncation would turn into
    // `Stay` (2^32) or into `South` (2^32 + 1).
    for to in [
        pos(edge + 1, 0, 0),
        pos(1 << 32, 0, 0),
        pos((1 << 32) + 1, 0, 0),
        pos(0, max, 0),
    ] {
        match Action::between(pos(0, 0, 0), to) {
            Err(RuntimeWorldError::PositionsNotAdjacent { pos_0, pos1 }) => {
                assert_eq!((pos_0, pos1), (pos(0, 0, 0), to));
            }
            other => panic!("{to:?}: expected PositionsNotAdjacent, got {other:?}"),
        }
    }
    assert!(matches!(
        Action::between(pos(max, max, 0), pos(0, 0, 0)),
        Err(RuntimeWorldError::PositionsNotAdjacent { .. })
    ));
}
