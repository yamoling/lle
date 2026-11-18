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
/// Exactly `norm(a) < limit`, but without computing `norm` when a coordinate of `a` alone is
/// too large.
///
/// `hypot` is within a few ulps of the exact norm, which is at least the absolute value of each
/// coordinate. So a coordinate above `|limit|` by the relative margin 1e-9, far larger than that
/// error, proves that the comparison is false. NaN coordinates go through `norm`.
///
/// @ai-generated
pub(super) fn norm_below(a: [f64; 2], limit: f64) -> bool {
    let bound = limit.abs() * (1.0 + 1e-9);
    if a[0].abs() > bound || a[1].abs() > bound {
        return false;
    }
    norm(a) < limit
}
/// Exactly `distance(p, cell) < limit` (see [`norm_below`]).
pub(super) fn distance_below(p: [f64; 2], cell: &Position, limit: f64) -> bool {
    norm_below(sub(p, closest(p, cell)), limit)
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
/// The distance along the ray from `origin` in the unit direction `direction` to the first
/// point of the axis-aligned box `[lo, hi]`, or 0 when `origin` is inside the box. `None` when
/// the ray misses the box.
///
/// @ai-generated
pub(super) fn ray_box(
    origin: [f64; 2],
    direction: [f64; 2],
    lo: [f64; 2],
    hi: [f64; 2],
) -> Option<f64> {
    let (mut t_min, mut t_max) = (0.0_f64, f64::INFINITY);
    for axis in 0..2 {
        if direction[axis] == 0.0 {
            if origin[axis] < lo[axis] || origin[axis] > hi[axis] {
                return None;
            }
        } else {
            let t1 = (lo[axis] - origin[axis]) / direction[axis];
            let t2 = (hi[axis] - origin[axis]) / direction[axis];
            t_min = t_min.max(t1.min(t2));
            t_max = t_max.min(t1.max(t2));
        }
    }
    (t_min <= t_max).then_some(t_min)
}
/// The distance along the ray from `origin` in the unit direction `direction` to the first
/// point of the disc of `centre` and `radius`, or 0 when `origin` is inside the disc. `None`
/// when the ray misses the disc.
///
/// @ai-generated
pub(super) fn ray_disc(
    origin: [f64; 2],
    direction: [f64; 2],
    centre: [f64; 2],
    radius: f64,
) -> Option<f64> {
    let m = sub(origin, centre);
    let c = dot(m, m) - radius * radius;
    if c <= 0.0 {
        return Some(0.0);
    }
    let b = dot(m, direction);
    if b > 0.0 {
        return None;
    }
    let discriminant = b * b - c;
    (discriminant >= 0.0).then(|| -b - discriminant.sqrt())
}
/// The distance along the ray from `origin` in the unit direction `direction` to the point
/// where it leaves the box `[lo, hi]`, or 0 when `origin` is not strictly inside the box.
///
/// @ai-generated
pub(super) fn ray_exit(origin: [f64; 2], direction: [f64; 2], lo: [f64; 2], hi: [f64; 2]) -> f64 {
    if (0..2).any(|d| origin[d] <= lo[d] || origin[d] >= hi[d]) {
        return 0.0;
    }
    (0..2)
        .filter_map(|d| match direction[d] {
            v if v > 0.0 => Some((hi[d] - origin[d]) / v),
            v if v < 0.0 => Some((lo[d] - origin[d]) / v),
            _ => None,
        })
        .fold(f64::INFINITY, f64::min)
}
