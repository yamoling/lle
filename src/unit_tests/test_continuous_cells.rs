use rand::{RngExt, SeedableRng, rngs::StdRng};

use super::CellIndex;
use crate::continuous::geometry::{distance, norm, norm_below};
use crate::shared::Position;

fn pos(i: usize, j: usize) -> Position {
    Position { i, j, k: 0 }
}

/// A 7×5 grid with a few solids (one of them twice), voids and exits.
fn index() -> (Vec<Position>, Vec<Position>, Vec<Position>, CellIndex) {
    let solids = vec![
        pos(0, 0),
        pos(2, 3),
        pos(2, 4),
        pos(4, 6),
        pos(2, 3),
        pos(3, 0),
    ];
    let voids = vec![pos(1, 1), pos(4, 5), pos(0, 6)];
    let exits = vec![pos(4, 0), pos(1, 3)];
    let index = CellIndex::new(7, 5, &solids, &voids, &exits);
    (solids, voids, exits, index)
}

/// Every solid and void that a disc of radius below 0.5 touches is listed near its centre, the
/// solids in increasing order, for centres inside and outside the grid.
#[test]
fn near_lists_contain_every_touching_cell() {
    let (solids, voids, _, index) = index();
    let mut rng = StdRng::seed_from_u64(0);
    for _ in 0..100_000 {
        let p = [rng.random_range(-1.0..8.0), rng.random_range(-1.0..6.0)];
        let near = index.solids_near(p);
        assert!(near.windows(2).all(|w| w[0] < w[1]), "{near:?}");
        for (s, solid) in solids.iter().enumerate() {
            if distance(p, solid) < 0.5 {
                assert!(near.contains(&(s as u32)), "solid {s} near {p:?}");
            }
        }
        let near = index.voids_near(p);
        for void in &voids {
            if distance(p, void) < 0.5 {
                assert!(near.contains(void), "void {void:?} near {p:?}");
            }
        }
    }
}

/// `in_exit` is the half-open cell test of every exit, on and around the cell edges.
#[test]
fn in_exit_matches_the_cell_test() {
    let (_, _, exits, index) = index();
    let coordinates: Vec<f64> = (-2..=16)
        .flat_map(|k| {
            let x = k as f64 / 2.0;
            [x.next_down(), x, x.next_up()]
        })
        .collect();
    for &x in &coordinates {
        for &y in &coordinates {
            let expected = exits.iter().any(|p| {
                x >= p.j as f64 && x < (p.j + 1) as f64 && y >= p.i as f64 && y < (p.i + 1) as f64
            });
            assert_eq!(index.in_exit(x, y), expected, "({x}, {y})");
        }
    }
}

/// `norm_below` is exactly `norm(a) < limit`, close to the limit too.
#[test]
fn norm_below_is_the_norm_comparison() {
    let mut rng = StdRng::seed_from_u64(1);
    for _ in 0..100_000 {
        let limit: f64 = rng.random_range(-0.1..1.0);
        let a = [rng.random_range(-1.0..1.0), rng.random_range(-1.0..1.0)];
        assert_eq!(norm_below(a, limit), norm(a) < limit, "{a:?} {limit}");
        // On the axes, the norm is the absolute value of the coordinate.
        for v in [limit.next_down(), limit, limit.next_up()] {
            for a in [[v, 0.0], [0.0, -v]] {
                assert_eq!(norm_below(a, limit), norm(a) < limit, "{a:?} {limit}");
            }
        }
    }
    assert!(!norm_below([f64::NAN, 0.0], 1.0));
}
