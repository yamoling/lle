use lle::Position;
use lle::solver::position_set::PositionSet;
use std::collections::HashSet;

fn pos(i: usize, j: usize) -> Position {
    Position { i, j }
}

#[test]
fn test_empty() {
    let set = PositionSet::empty(3, 3);
    assert!(set.is_empty());
    assert_eq!(set.iter().count(), 0);
    assert!(!set.contains(&pos(0, 0)));
}

#[test]
fn test_singleton() {
    let set = PositionSet::singleton(3, 3, pos(1, 2));
    assert!(!set.is_empty());
    assert!(set.contains(&pos(1, 2)));
    assert!(!set.contains(&pos(0, 0)));
    assert_eq!(set.iter().collect::<Vec<_>>(), vec![pos(1, 2)]);
}

#[test]
fn test_insert_and_contains() {
    let mut set = PositionSet::empty(4, 4);
    set.insert(pos(0, 0));
    set.insert(pos(3, 3));
    set.insert(pos(2, 1));

    assert!(set.contains(&pos(0, 0)));
    assert!(set.contains(&pos(3, 3)));
    assert!(set.contains(&pos(2, 1)));
    assert!(!set.contains(&pos(1, 1)));
    assert!(!set.is_empty());
}

#[test]
fn test_remove() {
    let mut set = PositionSet::empty(4, 4);
    set.insert(pos(1, 1));
    set.insert(pos(2, 2));
    assert!(set.contains(&pos(1, 1)));

    set.remove(&pos(1, 1));
    assert!(!set.contains(&pos(1, 1)));
    assert!(set.contains(&pos(2, 2)));

    // Removing a non-present position is a no-op.
    set.remove(&pos(0, 0));
    assert!(!set.contains(&pos(0, 0)));
}

#[test]
fn test_iter_matches_inserted_positions() {
    let mut set = PositionSet::empty(5, 5);
    let inserted: Vec<Position> = vec![pos(0, 0), pos(0, 4), pos(2, 2), pos(4, 4), pos(3, 1)];
    for p in &inserted {
        set.insert(*p);
    }

    let collected: HashSet<Position> = set.iter().collect();
    let expected: HashSet<Position> = inserted.into_iter().collect();
    assert_eq!(collected, expected);
}

#[test]
fn test_into_iter_owned() {
    let mut set = PositionSet::empty(3, 3);
    set.insert(pos(0, 1));
    set.insert(pos(2, 2));

    let collected: HashSet<Position> = set.into_iter().collect();
    let expected: HashSet<Position> = vec![pos(0, 1), pos(2, 2)].into_iter().collect();
    assert_eq!(collected, expected);
}

#[test]
fn test_iter_by_reference() {
    let mut set = PositionSet::empty(3, 3);
    set.insert(pos(1, 1));

    let collected: Vec<Position> = (&set).into_iter().collect();
    assert_eq!(collected, vec![pos(1, 1)]);
    // `set` is still usable afterwards since we iterated by reference.
    assert!(set.contains(&pos(1, 1)));
}

#[test]
fn test_intersect_with() {
    let mut a = PositionSet::empty(4, 4);
    let mut b = PositionSet::empty(4, 4);

    a.insert(pos(0, 0));
    a.insert(pos(1, 1));
    a.insert(pos(2, 2));

    b.insert(pos(1, 1));
    b.insert(pos(2, 2));
    b.insert(pos(3, 3));

    a.intersect_with(&b);

    assert!(!a.contains(&pos(0, 0)));
    assert!(a.contains(&pos(1, 1)));
    assert!(a.contains(&pos(2, 2)));
    assert!(!a.contains(&pos(3, 3)));
}

#[test]
fn test_intersect_with_disjoint_yields_empty() {
    let mut a = PositionSet::empty(4, 4);
    let mut b = PositionSet::empty(4, 4);

    a.insert(pos(0, 0));
    b.insert(pos(1, 1));

    a.intersect_with(&b);
    assert!(a.is_empty());
}

#[test]
fn test_subtract() {
    let mut a = PositionSet::empty(4, 4);
    let mut b = PositionSet::empty(4, 4);

    a.insert(pos(0, 0));
    a.insert(pos(1, 1));
    a.insert(pos(2, 2));

    b.insert(pos(1, 1));
    b.insert(pos(3, 3));

    a.subtract(&b);

    assert!(a.contains(&pos(0, 0)));
    assert!(!a.contains(&pos(1, 1)));
    assert!(a.contains(&pos(2, 2)));
}

#[test]
fn test_retain() {
    let mut set = PositionSet::empty(4, 4);
    set.insert(pos(0, 0));
    set.insert(pos(1, 1));
    set.insert(pos(2, 2));
    set.insert(pos(3, 3));

    // Keep only positions on rows >= 2.
    set.retain(|p| p.i >= 2);

    assert!(!set.contains(&pos(0, 0)));
    assert!(!set.contains(&pos(1, 1)));
    assert!(set.contains(&pos(2, 2)));
    assert!(set.contains(&pos(3, 3)));
}

#[test]
fn test_retain_all_false_yields_empty() {
    let mut set = PositionSet::empty(3, 3);
    set.insert(pos(0, 0));
    set.insert(pos(1, 1));

    set.retain(|_| false);
    assert!(set.is_empty());
}

#[test]
fn test_lazy_intersection() {
    let mut a = PositionSet::empty(4, 4);
    let mut b = PositionSet::empty(4, 4);

    a.insert(pos(0, 0));
    a.insert(pos(1, 1));
    a.insert(pos(2, 2));

    b.insert(pos(1, 1));
    b.insert(pos(2, 2));
    b.insert(pos(3, 3));

    let collected: HashSet<Position> = a.intersection(&b).collect();
    let expected: HashSet<Position> = vec![pos(1, 1), pos(2, 2)].into_iter().collect();
    assert_eq!(collected, expected);

    // The lazy intersection must not mutate either operand.
    assert!(a.contains(&pos(0, 0)));
    assert!(b.contains(&pos(3, 3)));
}

#[test]
fn test_positions_spanning_multiple_words() {
    // 10x10 grid -> 100 bits, spans across multiple u64 words (64 bits each).
    let height = 10;
    let width = 10;
    let mut set = PositionSet::empty(height, width);

    let mut expected = HashSet::new();
    for i in 0..height {
        for j in 0..width {
            if (i * width + j) % 7 == 0 {
                let p = pos(i, j);
                set.insert(p);
                expected.insert(p);
            }
        }
    }

    let collected: HashSet<Position> = set.iter().collect();
    assert_eq!(collected, expected);

    for i in 0..height {
        for j in 0..width {
            let p = pos(i, j);
            assert_eq!(set.contains(&p), expected.contains(&p));
        }
    }
}
