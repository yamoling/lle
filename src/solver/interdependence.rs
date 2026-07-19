use std::collections::HashSet;

use crate::AgentId;

/// A directed helper-to-beneficiary arc without a timestamp.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct StaticHelpArc {
    pub helper: AgentId,
    pub beneficiary: AgentId,
}

/// A chronologically rooted, splice-irreducible static closed-trail word.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ClosedTrailPattern {
    pub arcs: Vec<StaticHelpArc>,
    /// For each arc position, the positive prefix length of its preceding equal arc, if any.
    pub previous_same_prefix_len: Vec<Option<usize>>,
}

/// Return the largest edge count needed by an irreducible exact-order closed trail.
/// This bound is only defined for `k` >= 2.
///
/// @ai-generated
///
/// Bound computation:
/// $$
/// B(k)=\left\lfloor\frac{(k+1)^2}{4}\right\rfloor
/// $$
pub fn max_irreducible_closed_trail_len(order: usize) -> Option<usize> {
    if order < 2 {
        return None;
    }
    Some((order + 1) * (order + 1) / 4)
}

/// Enumerate every rooted splice-irreducible static closed-trail pattern of an exact order.
///
/// Only agents in `helper_ids` may occur: every member of a closed help trail must provide at
/// least one outgoing help arc. The IDs are normalized so enumeration and pattern indices remain
/// deterministic when callers provide them out of order or with duplicates.
///
/// @ai-generated
pub fn enumerate_closed_trail_patterns(
    mut helper_ids: Vec<AgentId>,
    order: usize,
) -> Vec<ClosedTrailPattern> {
    helper_ids.sort_unstable();
    helper_ids.dedup();
    let Some(max_len) = max_irreducible_closed_trail_len(order) else {
        return Vec::new();
    };

    if order > helper_ids.len() {
        return Vec::new();
    }

    let mut patterns = Vec::new();
    for &root in &helper_ids {
        let mut vertices = vec![root];
        let mut arcs = Vec::new();
        enumerate_from(
            &helper_ids,
            order,
            max_len,
            root,
            &mut vertices,
            &mut arcs,
            &mut patterns,
        );
    }
    // Pattern indices are embedded in SAT progress keys, so their ordering must remain deterministic.
    patterns.sort_by(|left, right| left.arcs.cmp(&right.arcs));
    patterns
}

/// Extend one rooted static trail word while enforcing the irreducible-witness bounds.
///
/// @ai-generated
fn enumerate_from(
    helper_ids: &[AgentId],
    order: usize,
    max_len: usize,
    root: AgentId,
    vertices: &mut Vec<AgentId>,
    arcs: &mut Vec<StaticHelpArc>,
    patterns: &mut Vec<ClosedTrailPattern>,
) {
    if arcs.len() == max_len {
        return;
    }
    let current = *vertices
        .last()
        .expect("a rooted word always has a current vertex");
    let arc_limit = order.saturating_sub(2).max(1);
    for &next in helper_ids {
        if next == current {
            continue;
        }
        let arc = StaticHelpArc {
            helper: current,
            beneficiary: next,
        };
        if arcs.iter().filter(|&&existing| existing == arc).count() >= arc_limit {
            continue;
        }

        // A return to the root can complete a pattern -> temporarily close the trail and emit it
        // when it has exact support and is irreducible, then restore the prefix for further DFS.
        if next == root && arcs.len() + 1 >= order {
            arcs.push(arc);
            if has_exact_support(arcs, order) && !is_splice_reducible(arcs, vertices) {
                patterns.push(ClosedTrailPattern {
                    previous_same_prefix_len: previous_same_prefix_lens(arcs),
                    arcs: arcs.clone(),
                });
            }
            arcs.pop();
        }

        let occurrences = vertices.iter().filter(|&&agent| agent == next).count();
        if occurrences >= order - 1 {
            continue;
        }
        let mut support = vertices.iter().copied().collect::<HashSet<_>>();
        support.insert(next);
        if support.len() > order {
            continue;
        }
        let missing = order - support.len();
        // Each missing agent needs one entry edge, and one further edge is needed to close.
        if arcs.len() + 1 + missing + 1 > max_len {
            continue;
        }
        arcs.push(arc);
        vertices.push(next);
        enumerate_from(helper_ids, order, max_len, root, vertices, arcs, patterns);
        vertices.pop();
        arcs.pop();
    }
}

/// Test whether the endpoints of `arcs` contain exactly `order` distinct agents.
///
/// @ai-generated
fn has_exact_support(arcs: &[StaticHelpArc], order: usize) -> bool {
    arcs.iter()
        .flat_map(|arc| [arc.helper, arc.beneficiary])
        .collect::<HashSet<_>>()
        .len()
        == order
}

/// Reject a rooted word if an equal cut point permits full-support extraction or splicing.
///
/// @ai-generated
///
/// For example:
/// - **Extraction:** in `0 → 1 → 2 → 0 → 1 → 0`, the segment between the first two
///   occurrences of `0` is the closed subtrail `0 → 1 → 2 → 0`, which retains full support.
/// - **Splicing:** in `0 → 1 → 0 → 1 → 2 → 1 → 0`, splicing out the inner `1 → 0 → 1`
///   segment leaves the full-support closed trail `0 → 1 → 2 → 1 → 0` across the rooted cut.
fn is_splice_reducible(arcs: &[StaticHelpArc], vertices: &[AgentId]) -> bool {
    debug_assert_eq!(vertices.len(), arcs.len());
    let order = arcs
        .iter()
        .flat_map(|arc| [arc.helper, arc.beneficiary])
        .collect::<HashSet<_>>()
        .len();
    for start in 0..vertices.len() {
        for end in start + 1..=vertices.len() {
            let end_vertex = if end == vertices.len() {
                vertices[0]
            } else {
                vertices[end]
            };
            if vertices[start] != end_vertex || (start == 0 && end == vertices.len()) {
                continue;
            }
            let inner = &arcs[start..end];
            if inner.len() < arcs.len() && has_exact_support(inner, order) {
                return true;
            }
            let mut outer = Vec::with_capacity(arcs.len() - inner.len());
            outer.extend_from_slice(&arcs[..start]);
            outer.extend_from_slice(&arcs[end..]);
            if !outer.is_empty() && has_exact_support(&outer, order) {
                return true;
            }
        }
    }
    false
}

/// Precompute the preceding equal-static-arc prefix for each position.
///
/// @ai-generated
///
/// Reusing a static arc is valid only at a strictly later timestamp: otherwise both occurrences
/// would select the same temporal edge. The Horn progress encoding uses this table to add the
/// `progress(previous_prefix, t - 1)` prerequisite for a repeated arc, while allowing distinct
/// arcs to occur at the same time.
///
/// For example, `a → b, b → c, a → b` produces `[None, None, Some(1)]`: the last `a → b`
/// repeats the arc at zero-based index `0`, whose completed prefix has length `1`.
fn previous_same_prefix_lens(arcs: &[StaticHelpArc]) -> Vec<Option<usize>> {
    arcs.iter()
        .enumerate()
        .map(|(index, &arc)| {
            arcs[..index]
                .iter()
                .rposition(|&previous| previous == arc)
                .map(|previous_index| previous_index + 1)
        })
        .collect()
}

#[cfg(test)]
#[path = "../unit_tests/engine/test_interdependence.rs"]
mod tests;
