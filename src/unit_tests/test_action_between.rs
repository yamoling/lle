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
fn between_cells_on_different_layers_is_an_error() {
    // Same (i, j) on another layer, and a planar neighbour on another layer.
    let from = pos(1, 1, 0);
    for to in [pos(1, 1, 1), pos(0, 1, 1), pos(1, 2, 2)] {
        assert!(Action::between(from, to).is_err(), "{to:?}");
        assert!(Action::between(to, from).is_err(), "reversed {to:?}");
    }
}
