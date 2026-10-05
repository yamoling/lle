use crate::{Position, core::boxes::Boxes};

fn pos(i: usize, j: usize) -> Position {
    Position { i, j }
}

fn two_boxes() -> Boxes {
    Boxes::new(vec![pos(0, 0), pos(1, 1)], 3, 3)
}

#[test]
fn test_new_boxes_are_present_at_their_initial_positions() {
    let boxes = two_boxes();
    assert_eq!(boxes.len(), 2);
    assert_eq!(boxes.positions(), &vec![pos(0, 0), pos(1, 1)]);
    assert_eq!(boxes.present(), &vec![true, true]);
    assert_eq!(boxes.id_at(pos(0, 0)), Some(0));
    assert_eq!(boxes.id_at(pos(1, 1)), Some(1));
    assert_eq!(boxes.id_at(pos(2, 2)), None);
}

#[test]
fn test_set_position_moves_the_occupancy_index() {
    let mut boxes = two_boxes();
    boxes.set_position(0, pos(0, 1));
    assert_eq!(boxes.id_at(pos(0, 0)), None);
    assert_eq!(boxes.id_at(pos(0, 1)), Some(0));
    assert_eq!(boxes.positions()[0], pos(0, 1));
}

#[test]
fn test_destroyed_box_frees_its_cell() {
    let mut boxes = two_boxes();
    boxes.destroy(1);
    assert!(!boxes.present()[1]);
    assert_eq!(
        boxes.id_at(pos(1, 1)),
        None,
        "a destroyed box occupies nothing"
    );
    assert_eq!(
        boxes.positions()[1],
        pos(1, 1),
        "but keeps its last position"
    );
}

#[test]
fn test_reset_restores_positions_and_revives_destroyed_boxes() {
    let mut boxes = two_boxes();
    boxes.set_position(0, pos(2, 2));
    boxes.destroy(1);

    boxes.reset();

    assert_eq!(boxes.positions(), &vec![pos(0, 0), pos(1, 1)]);
    assert_eq!(boxes.present(), &vec![true, true]);
    assert_eq!(boxes.id_at(pos(2, 2)), None);
    assert_eq!(boxes.id_at(pos(0, 0)), Some(0));
    assert_eq!(boxes.id_at(pos(1, 1)), Some(1));
}

#[test]
fn test_restore_sets_an_arbitrary_consistent_state() {
    let mut boxes = two_boxes();
    boxes.restore(&[pos(2, 0), pos(2, 1)], &[true, false]);
    assert_eq!(boxes.id_at(pos(2, 0)), Some(0));
    assert_eq!(boxes.id_at(pos(2, 1)), None, "absent boxes hold no cell");
    assert_eq!(boxes.positions(), &vec![pos(2, 0), pos(2, 1)]);
}

#[test]
fn test_id_at_returns_none_for_position_outside_grid() {
    let boxes = two_boxes();
    // Grid is 3x3, so valid indices are 0-2 for both i and j
    assert_eq!(
        boxes.id_at(pos(3, 0)),
        None,
        "position outside grid returns None"
    );
    assert_eq!(
        boxes.id_at(pos(0, 3)),
        None,
        "position outside grid returns None"
    );
    assert_eq!(
        boxes.id_at(pos(5, 5)),
        None,
        "position far outside grid returns None"
    );
}
