use crate::shared::Position;
pub(super) fn centre(p: &Position) -> [f64; 2] {
    [p.j as f64 + 0.5, p.i as f64 + 0.5]
}
pub(super) fn sub(a: [f64; 2], b: [f64; 2]) -> [f64; 2] {
    [a[0] - b[0], a[1] - b[1]]
}
pub(super) fn norm(a: [f64; 2]) -> f64 {
    a[0].hypot(a[1])
}
pub(super) fn dot(a: [f64; 2], b: [f64; 2]) -> f64 {
    a[0] * b[0] + a[1] * b[1]
}
pub(super) fn clip(a: [f64; 2], max: f64) -> [f64; 2] {
    let n = norm(a);
    if n > max {
        [a[0] / n * max, a[1] / n * max]
    } else {
        a
    }
}
pub(super) fn closest(p: [f64; 2], cell: &Position) -> [f64; 2] {
    [
        p[0].clamp(cell.j as f64, cell.j as f64 + 1.0),
        p[1].clamp(cell.i as f64, cell.i as f64 + 1.0),
    ]
}
pub(super) fn distance(p: [f64; 2], cell: &Position) -> f64 {
    norm(sub(p, closest(p, cell)))
}
