//! Eager enumeration of static patterns that may witness temporal interdependence.
//!
//! The SAT encoding cannot directly quantify over arbitrary closed trails. Before generating clauses,
//! this module therefore enumerates a finite basis of static helper-to-beneficiary arc words. The
//! clause layer later asks whether the corresponding `Help` events can occur in chronological order.
//! This is an **eager** approach: the complete static basis is built before SAT solving starts.
//!
//! ## Intuition: search once, restore chronology, backtrack cheaply
//!
//! A closed trail is circular and has no natural first edge. For example, these are the same circular
//! trail:
//!
//! ```text
//! 0 → 1 → 2 → 0
//! 1 → 2 → 0 → 1
//! 2 → 0 → 1 → 2
//! ```
//!
//! The enumerator avoids rediscovering that circle independently from every possible starting point:
//!
//! 1. **Search one canonical circular representative.** A DFS rooted at agent `r` may visit only
//!    agents whose IDs are at least `r`, so `r` must be the smallest agent in the trail's support.
//!    If that smallest agent occurs several times, only the lexicographically least cyclic rotation
//!    is accepted. Together, these rules make each circular trail class expensive to discover only
//!    once.
//! 2. **Regenerate every chronological rooting.** The SAT progress encoding still needs every
//!    distinct rotation: rotating the word changes where a temporal sequence may wrap from a late
//!    timestamp back to an early one. Once the canonical circle is found, constructing its rotations
//!    is cheap. Repeated-arc metadata is recomputed after every rotation because the chronological
//!    cut changes which occurrence is considered previous.
//! 3. **Backtrack with reversible counters.** During DFS, dense arrays track how often each agent and
//!    static arc occurs, plus the current number of supported agents. Extending a prefix increments a
//!    few counters; backtracking decrements them. This replaces repeated scans and temporary
//!    `HashSet` construction at every search node.
//!
//! The remaining filters bound the trail length and multiplicities, keep the search inside the root's
//! strongly connected region, require exact support, and reject splice-reducible words. Finally, the
//! regenerated rooted patterns are sorted so their SAT pattern indices are deterministic.
//!
//! These optimizations reduce DFS work and per-node cost only. They deliberately preserve the exact
//! rooted pattern basis, clause count, variable count, and `NoInterdependence` semantics. On a dense
//! potential-help graph, the eager basis or the search needed to find it may still be very large.

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
/// occur within the solver horizon. Each cyclic class is searched only from its minimum support
/// agent; every distinct chronological rotation is regenerated when the canonical class is found.
/// Traversal is further restricted to agents that are both reachable from the root and can return to
/// it. Inputs and outputs are normalized so SAT pattern indices remain deterministic.
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

    let graph = SearchGraph::new(&potential_arcs);
    let mut roots = graph.outgoing.keys().copied().collect::<Vec<_>>();
    roots.sort_unstable();
    if order > roots.len() {
        return Vec::new();
    }

    let mut patterns = Vec::new();
    for root in roots {
        let forward = graph.reachable_agents(root, false);
        let backward = graph.reachable_agents(root, true);
        let closed_component = forward
            .intersection(&backward)
            .copied()
            .collect::<HashSet<_>>();
        if closed_component.len() < order {
            continue;
        }

        let search = TrailSearch {
            graph: &graph,
            closed_component: &closed_component,
            order,
            max_len,
            root,
        };
        let mut state = TrailState::new(&graph, root);
        search.enumerate_from(&mut state, &mut patterns);
    }
    patterns.sort_by(|left, right| left.arcs.cmp(&right.arcs));
    patterns
}

#[derive(Clone, Copy)]
struct IndexedHelpArc {
    arc: StaticHelpArc,
    beneficiary_index: usize,
    arc_index: usize,
}

/// Dense graph representation used by reversible DFS bookkeeping.
struct SearchGraph {
    outgoing: HashMap<AgentId, Vec<IndexedHelpArc>>,
    incoming: HashMap<AgentId, Vec<AgentId>>,
    agent_indices: HashMap<AgentId, usize>,
    n_arcs: usize,
}

impl SearchGraph {
    /// Index normalized agents and arcs while preserving deterministic adjacency order.
    ///
    /// @ai-generated
    fn new(potential_arcs: &[StaticHelpArc]) -> Self {
        let mut agents = potential_arcs
            .iter()
            .flat_map(|arc| [arc.helper, arc.beneficiary])
            .collect::<Vec<_>>();
        agents.sort_unstable();
        agents.dedup();
        let agent_indices = agents
            .into_iter()
            .enumerate()
            .map(|(index, agent)| (agent, index))
            .collect::<HashMap<_, _>>();
        let arc_indices = potential_arcs
            .iter()
            .copied()
            .enumerate()
            .map(|(index, arc)| (arc, index))
            .collect::<HashMap<_, _>>();
        let mut outgoing = HashMap::<AgentId, Vec<IndexedHelpArc>>::new();
        let mut incoming = HashMap::<AgentId, Vec<AgentId>>::new();
        for &arc in potential_arcs {
            outgoing
                .entry(arc.helper)
                .or_default()
                .push(IndexedHelpArc {
                    arc,
                    beneficiary_index: agent_indices[&arc.beneficiary],
                    arc_index: arc_indices[&arc],
                });
            incoming
                .entry(arc.beneficiary)
                .or_default()
                .push(arc.helper);
        }
        Self {
            outgoing,
            incoming,
            agent_indices,
            n_arcs: potential_arcs.len(),
        }
    }

    /// Return every agent reachable from `start` in the forward or reverse graph.
    ///
    /// @ai-generated
    fn reachable_agents(&self, start: AgentId, reverse: bool) -> HashSet<AgentId> {
        let mut reachable = HashSet::from([start]);
        let mut pending = vec![start];
        while let Some(current) = pending.pop() {
            if reverse {
                for &next in self.incoming.get(&current).into_iter().flatten() {
                    if reachable.insert(next) {
                        pending.push(next);
                    }
                }
            } else {
                for indexed in self.outgoing.get(&current).into_iter().flatten() {
                    if reachable.insert(indexed.arc.beneficiary) {
                        pending.push(indexed.arc.beneficiary);
                    }
                }
            }
        }
        reachable
    }
}

/// Reversible per-prefix state with constant-time multiplicity and support updates.
struct TrailState {
    vertices: Vec<AgentId>,
    arcs: Vec<StaticHelpArc>,
    agent_occurrences: Vec<usize>,
    arc_occurrences: Vec<usize>,
    support_size: usize,
}

impl TrailState {
    fn new(graph: &SearchGraph, root: AgentId) -> Self {
        let mut agent_occurrences = vec![0; graph.agent_indices.len()];
        agent_occurrences[graph.agent_indices[&root]] = 1;
        Self {
            vertices: vec![root],
            arcs: Vec::new(),
            agent_occurrences,
            arc_occurrences: vec![0; graph.n_arcs],
            support_size: 1,
        }
    }

    /// Append one indexed arc and update all dense counters.
    ///
    /// @ai-generated
    fn push(&mut self, indexed: IndexedHelpArc) {
        if self.agent_occurrences[indexed.beneficiary_index] == 0 {
            self.support_size += 1;
        }
        self.agent_occurrences[indexed.beneficiary_index] += 1;
        self.arc_occurrences[indexed.arc_index] += 1;
        self.vertices.push(indexed.arc.beneficiary);
        self.arcs.push(indexed.arc);
    }

    /// Remove the most recently appended indexed arc and restore all counters.
    ///
    /// @ai-generated
    fn pop(&mut self, indexed: IndexedHelpArc) {
        debug_assert_eq!(self.arcs.last(), Some(&indexed.arc));
        self.arcs.pop();
        self.vertices.pop();
        self.arc_occurrences[indexed.arc_index] -= 1;
        self.agent_occurrences[indexed.beneficiary_index] -= 1;
        if self.agent_occurrences[indexed.beneficiary_index] == 0 {
            self.support_size -= 1;
        }
    }
}

/// Immutable graph and bounds used while enumerating trails from one canonical root.
struct TrailSearch<'a> {
    graph: &'a SearchGraph,
    closed_component: &'a HashSet<AgentId>,
    order: usize,
    max_len: usize,
    root: AgentId,
}

impl TrailSearch<'_> {
    /// Extend one rooted static trail word while enforcing the irreducible-witness bounds.
    ///
    /// @ai-generated
    fn enumerate_from(&self, state: &mut TrailState, patterns: &mut Vec<ClosedTrailPattern>) {
        if state.arcs.len() == self.max_len {
            return;
        }
        let current = *state
            .vertices
            .last()
            .expect("a rooted word always has a current vertex");
        let arc_limit = self.order.saturating_sub(2).max(1);
        for &indexed in self.graph.outgoing.get(&current).into_iter().flatten() {
            let next = indexed.arc.beneficiary;
            if next < self.root
                || !self.closed_component.contains(&next)
                || state.arc_occurrences[indexed.arc_index] >= arc_limit
            {
                continue;
            }

            // A return to the root can complete a pattern. The duplicate terminal root is not a new
            // source occurrence, so only the arc word is temporarily extended for validation.
            if next == self.root && state.arcs.len() + 1 >= self.order {
                state.arcs.push(indexed.arc);
                if state.support_size == self.order
                    && !is_splice_reducible(&state.arcs, &state.vertices)
                    && is_canonical_rotation(&state.arcs)
                {
                    append_distinct_rotations(&state.arcs, patterns);
                }
                state.arcs.pop();
            }

            if state.agent_occurrences[indexed.beneficiary_index] >= self.order - 1 {
                continue;
            }
            let next_support_size = state.support_size
                + usize::from(state.agent_occurrences[indexed.beneficiary_index] == 0);
            if next_support_size > self.order {
                continue;
            }
            let missing = self.order - next_support_size;
            // Each missing agent needs one entry edge, and one further edge is needed to close.
            if state.arcs.len() + 1 + missing + 1 > self.max_len {
                continue;
            }
            state.push(indexed);
            self.enumerate_from(state, patterns);
            state.pop(indexed);
        }
    }
}

/// Test whether `arcs` is the lexicographically least cyclic rotation of its class.
///
/// @ai-generated
fn is_canonical_rotation(arcs: &[StaticHelpArc]) -> bool {
    (1..arcs.len()).all(|cut| {
        arcs.iter().cmp(arcs[cut..].iter().chain(&arcs[..cut])) != std::cmp::Ordering::Greater
    })
}

/// Append every distinct chronological rotation with rotation-local repeated-arc metadata.
///
/// @ai-generated
fn append_distinct_rotations(arcs: &[StaticHelpArc], patterns: &mut Vec<ClosedTrailPattern>) {
    let mut rotations = (0..arcs.len())
        .map(|cut| {
            let rotated = arcs[cut..]
                .iter()
                .chain(&arcs[..cut])
                .copied()
                .collect::<Vec<_>>();
            ClosedTrailPattern {
                previous_same_prefix_len: previous_same_prefix_lens(&rotated),
                arcs: rotated,
            }
        })
        .collect::<Vec<_>>();
    rotations.sort_by(|left, right| left.arcs.cmp(&right.arcs));
    rotations.dedup_by(|left, right| left.arcs == right.arcs);
    patterns.extend(rotations);
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
