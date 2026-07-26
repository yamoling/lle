use std::collections::{HashMap, HashSet};

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
/// Enumeration follows only `potential_arcs`, which must be help relationships that can physically
/// occur within the solver horizon. For each root, traversal is further restricted to agents that
/// are both reachable from the root and can return to it; no other agent can belong to a closed
/// trail rooted there. Inputs and outputs are normalized so SAT pattern indices remain deterministic.
///
/// @ai-generated
pub fn enumerate_closed_trail_patterns(
    mut potential_arcs: Vec<StaticHelpArc>,
    order: usize,
) -> Vec<ClosedTrailPattern> {
    let Some(max_len) = max_irreducible_closed_trail_len(order) else {
        return Vec::new();
    };
    potential_arcs.retain(|arc| arc.helper != arc.beneficiary);
    potential_arcs.sort_unstable();
    potential_arcs.dedup();

    let mut outgoing = HashMap::<AgentId, Vec<AgentId>>::new();
    let mut incoming = HashMap::<AgentId, Vec<AgentId>>::new();
    for arc in potential_arcs {
        outgoing
            .entry(arc.helper)
            .or_default()
            .push(arc.beneficiary);
        incoming
            .entry(arc.beneficiary)
            .or_default()
            .push(arc.helper);
    }
    let mut roots = outgoing.keys().copied().collect::<Vec<_>>();
    roots.sort_unstable();
    if order > roots.len() {
        return Vec::new();
    }

    let mut patterns = Vec::new();
    for root in roots {
        let forward = reachable_agents(root, &outgoing);
        let backward = reachable_agents(root, &incoming);
        let closed_component = forward
            .intersection(&backward)
            .copied()
            .collect::<HashSet<_>>();
        if closed_component.len() < order {
            continue;
        }

        let search = TrailSearch {
            outgoing: &outgoing,
            closed_component: &closed_component,
            order,
            max_len,
            root,
        };
        let mut vertices = vec![root];
        let mut arcs = Vec::new();
        search.enumerate_from(&mut vertices, &mut arcs, &mut patterns);
    }
    patterns.sort_by(|left, right| left.arcs.cmp(&right.arcs));
    patterns
}

/// Return every agent reachable from `start` through the supplied adjacency lists.
///
/// @ai-generated
fn reachable_agents(
    start: AgentId,
    adjacency: &HashMap<AgentId, Vec<AgentId>>,
) -> HashSet<AgentId> {
    let mut reachable = HashSet::from([start]);
    let mut pending = vec![start];
    while let Some(current) = pending.pop() {
        for &next in adjacency.get(&current).into_iter().flatten() {
            if reachable.insert(next) {
                pending.push(next);
            }
        }
    }
    reachable
}

/// Immutable graph and bounds used while enumerating trails from one root.
struct TrailSearch<'a> {
    outgoing: &'a HashMap<AgentId, Vec<AgentId>>,
    closed_component: &'a HashSet<AgentId>,
    order: usize,
    max_len: usize,
    root: AgentId,
}

impl TrailSearch<'_> {
    /// Extend one rooted static trail word while enforcing the irreducible-witness bounds.
    ///
    /// @ai-generated
    fn enumerate_from(
        &self,
        vertices: &mut Vec<AgentId>,
        arcs: &mut Vec<StaticHelpArc>,
        patterns: &mut Vec<ClosedTrailPattern>,
    ) {
        if arcs.len() == self.max_len {
            return;
        }
        let current = *vertices
            .last()
            .expect("a rooted word always has a current vertex");
        let arc_limit = self.order.saturating_sub(2).max(1);
        for &next in self.outgoing.get(&current).into_iter().flatten() {
            if !self.closed_component.contains(&next) {
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
            if next == self.root && arcs.len() + 1 >= self.order {
                arcs.push(arc);
                if has_exact_support(arcs, self.order) && !is_splice_reducible(arcs, vertices) {
                    patterns.push(ClosedTrailPattern {
                        previous_same_prefix_len: previous_same_prefix_lens(arcs),
                        arcs: arcs.clone(),
                    });
                }
                arcs.pop();
            }

            let occurrences = vertices.iter().filter(|&&agent| agent == next).count();
            if occurrences >= self.order - 1 {
                continue;
            }
            let mut support = vertices.iter().copied().collect::<HashSet<_>>();
            support.insert(next);
            if support.len() > self.order {
                continue;
            }
            let missing = self.order - support.len();
            // Each missing agent needs one entry edge, and one further edge is needed to close.
            if arcs.len() + 1 + missing + 1 > self.max_len {
                continue;
            }
            arcs.push(arc);
            vertices.push(next);
            self.enumerate_from(vertices, arcs, patterns);
            vertices.pop();
            arcs.pop();
        }
    }
}

/// Test whether the endpoints of `arcs` contain exactly `order` distinct agents.
fn has_exact_support(arcs: &[StaticHelpArc], order: usize) -> bool {
    arcs.iter()
        .flat_map(|arc| [arc.helper, arc.beneficiary])
        .collect::<HashSet<_>>()
        .len()
        == order
}

/// Reject a rooted word if an equal cut point permits full-support extraction or splicing.
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

#[cfg(test)]
#[path = "../../.agents/scratches/probe_interdependence.rs"]
mod probe;
