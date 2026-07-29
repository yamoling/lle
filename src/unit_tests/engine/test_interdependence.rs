use std::collections::{HashMap, HashSet};

use super::{
    ClosedTrailPattern, SearchGraph, StaticHelpArc, TrailSearch, TrailState,
    enumerate_closed_trail_patterns, is_splice_reducible, max_irreducible_closed_trail_len,
    previous_same_prefix_lens,
};
use crate::AgentId;
use rstest::rstest;

/// Build the complete loop-free directed help relation over `agents`.
fn complete_help_arcs(agents: &[AgentId]) -> Vec<StaticHelpArc> {
    agents
        .iter()
        .flat_map(|&helper| {
            agents
                .iter()
                .filter(move |&&beneficiary| beneficiary != helper)
                .map(move |&beneficiary| StaticHelpArc {
                    helper,
                    beneficiary,
                })
        })
        .collect()
}

/// Return one graph-indexed arc for direct DFS state tests.
///
/// @ai-generated
fn indexed_arc(graph: &SearchGraph, arc: StaticHelpArc) -> super::IndexedHelpArc {
    graph.outgoing[&arc.helper]
        .iter()
        .copied()
        .find(|indexed| indexed.arc == arc)
        .expect("test arc must belong to the search graph")
}

/// Convert a closed vertex word into the parallel arc and cut-point representations under test.
///
/// @ai-generated
fn closed_word(vertices: &[AgentId]) -> (Vec<StaticHelpArc>, Vec<AgentId>) {
    assert_eq!(vertices.first(), vertices.last());
    let arcs = vertices
        .windows(2)
        .map(|edge| StaticHelpArc {
            helper: edge[0],
            beneficiary: edge[1],
        })
        .collect();
    (arcs, vertices[..vertices.len() - 1].to_vec())
}

/// Reproduce the former every-root DFS as an exact test oracle.
///
/// @ai-generated
fn reference_patterns(
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
    let mut patterns = Vec::new();
    for root in roots {
        let forward = reference_reachable(root, &outgoing);
        let backward = reference_reachable(root, &incoming);
        let component = forward
            .intersection(&backward)
            .copied()
            .collect::<HashSet<_>>();
        if component.len() < order {
            continue;
        }
        reference_enumerate_from(
            &outgoing,
            &component,
            order,
            max_len,
            root,
            &mut vec![root],
            &mut Vec::new(),
            &mut patterns,
        );
    }
    patterns.sort_by(|left, right| left.arcs.cmp(&right.arcs));
    patterns
}

/// Return the reference graph's reachable agents.
///
/// @ai-generated
fn reference_reachable(
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

/// Extend one prefix using the former scan-based bookkeeping.
///
/// @ai-generated
#[allow(clippy::too_many_arguments)]
fn reference_enumerate_from(
    outgoing: &HashMap<AgentId, Vec<AgentId>>,
    component: &HashSet<AgentId>,
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
    let current = *vertices.last().unwrap();
    let arc_limit = order.saturating_sub(2).max(1);
    for &next in outgoing.get(&current).into_iter().flatten() {
        if !component.contains(&next) {
            continue;
        }
        let arc = StaticHelpArc {
            helper: current,
            beneficiary: next,
        };
        if arcs.iter().filter(|&&existing| existing == arc).count() >= arc_limit {
            continue;
        }
        if next == root && arcs.len() + 1 >= order {
            arcs.push(arc);
            if super::has_exact_support(arcs, order) && !is_splice_reducible(arcs, vertices) {
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
        if arcs.len() + 1 + missing + 1 > max_len {
            continue;
        }
        arcs.push(arc);
        vertices.push(next);
        reference_enumerate_from(
            outgoing, component, order, max_len, root, vertices, arcs, patterns,
        );
        vertices.pop();
        arcs.pop();
    }
}

/// Decode one loop-free digraph from a deterministic arc bitset.
///
/// @ai-generated
fn graph_from_bits(n_agents: usize, bits: u64) -> Vec<StaticHelpArc> {
    complete_help_arcs(&(0..n_agents).collect::<Vec<_>>())
        .into_iter()
        .enumerate()
        .filter_map(|(index, arc)| ((bits >> index) & 1 == 1).then_some(arc))
        .collect()
}

/// Empty and pairwise-distinct patterns have no preceding equal-static-arc prefixes.
///
/// @ai-generated
#[test]
fn previous_same_prefix_lens_returns_none_without_repeats() {
    assert!(previous_same_prefix_lens(&[]).is_empty());
    let arcs = [
        StaticHelpArc {
            helper: 0,
            beneficiary: 1,
        },
        StaticHelpArc {
            helper: 1,
            beneficiary: 2,
        },
        StaticHelpArc {
            helper: 2,
            beneficiary: 0,
        },
    ];
    assert_eq!(previous_same_prefix_lens(&arcs), vec![None, None, None]);
}

/// A non-adjacent repeat stores the one-based prefix length of its earlier occurrence.
///
/// @ai-generated
#[test]
fn previous_same_prefix_lens_records_non_adjacent_repeat() {
    let repeated = StaticHelpArc {
        helper: 0,
        beneficiary: 1,
    };
    let arcs = [
        repeated,
        StaticHelpArc {
            helper: 1,
            beneficiary: 2,
        },
        repeated,
    ];
    assert_eq!(previous_same_prefix_lens(&arcs), vec![None, None, Some(1)]);
}

/// Consecutive repetitions refer to the most recent matching arc rather than the first one.
///
/// @ai-generated
#[test]
fn previous_same_prefix_lens_uses_most_recent_repeat() {
    let repeated = StaticHelpArc {
        helper: 0,
        beneficiary: 1,
    };
    let arcs = [repeated, repeated, repeated];
    assert_eq!(
        previous_same_prefix_lens(&arcs),
        vec![None, Some(1), Some(2)]
    );
}

/// Full-support extraction and cut-crossing splicing both make a rooted word reducible.
///
/// @ai-generated
#[rstest]
#[case(&[0, 1, 2, 0, 1, 0], true)] // Can reduce to 0->1->2->0
#[case(&[0, 1, 0, 1, 2, 1, 0], true)] // Can reduce to 0->1->2->1->0
#[case(&[0, 1, 2, 0], false)] // A ring has no proper equal-cut-point segment
#[case(&[0, 1, 0, 2, 0], false)] // Canonical bowtie has no proper equal-cut-point segment
fn splice_reducibility_detects_nominal_reductions(
    #[case] word: &[AgentId],
    #[case] expected: bool,
) {
    let (arcs, vertices) = closed_word(word);
    assert_eq!(is_splice_reducible(&arcs, &vertices), expected);
}

/// Oversized support orders have no closed-trail patterns.
///
/// @ai-generated
#[test]
fn enumerate_closed_trails_too_large_order() {
    assert!(enumerate_closed_trail_patterns(complete_help_arcs(&[0, 1, 2, 3]), 5).is_empty());
}

/// Pattern enumeration uses only helper IDs and normalizes their order and duplicates.
///
/// @ai-generated
#[test]
fn enumerate_closed_trails_excludes_non_helpers() {
    let mut duplicated_arcs = complete_help_arcs(&[1, 3]);
    duplicated_arcs.push(StaticHelpArc {
        helper: 3,
        beneficiary: 1,
    });
    let patterns = enumerate_closed_trail_patterns(duplicated_arcs, 2);

    assert_eq!(
        patterns,
        enumerate_closed_trail_patterns(complete_help_arcs(&[1, 3]), 2)
    );
    assert_eq!(patterns.len(), 2);
    assert!(
        patterns
            .iter()
            .flat_map(|pattern| &pattern.arcs)
            .all(|arc| { [1, 3].contains(&arc.helper) && [1, 3].contains(&arc.beneficiary) })
    );
}

/// A sparse eight-agent ring emits only its eight rooted rotations, not complete-graph patterns.
///
/// @ai-generated
#[test]
fn sparse_eight_agent_ring_enumerates_only_feasible_closed_trails() {
    let ring = (0..8)
        .map(|helper| StaticHelpArc {
            helper,
            beneficiary: (helper + 1) % 8,
        })
        .collect::<Vec<_>>();

    let patterns = enumerate_closed_trail_patterns(ring.clone(), 8);

    assert_eq!(patterns.len(), 8);
    assert!(patterns.iter().all(|pattern| pattern.arcs.len() == 8));
    assert!(
        patterns
            .iter()
            .flat_map(|pattern| &pattern.arcs)
            .all(|arc| ring.contains(arc))
    );
}

/// Agents outside the root's strongly connected region cannot occur in a closed-trail pattern.
///
/// @ai-generated
#[test]
fn closed_trail_enumeration_prunes_one_way_branches() {
    let arcs = vec![
        StaticHelpArc {
            helper: 0,
            beneficiary: 1,
        },
        StaticHelpArc {
            helper: 1,
            beneficiary: 0,
        },
        StaticHelpArc {
            helper: 1,
            beneficiary: 2,
        },
    ];

    assert!(enumerate_closed_trail_patterns(arcs, 3).is_empty());
}

/// Starting from the canonical root emits every rotation and restores state.
///
/// @ai-generated
#[test]
fn enumerate_from_emits_rotations_and_restores_root_prefix() {
    let graph = SearchGraph::new(&complete_help_arcs(&[0, 1]));
    let component = HashSet::from([0, 1]);
    let mut state = TrailState::new(&graph, 0);
    let mut patterns = Vec::new();
    TrailSearch {
        graph: &graph,
        closed_component: &component,
        order: 2,
        max_len: 2,
        root: 0,
    }
    .enumerate_from(&mut state, &mut patterns);
    assert_eq!(state.vertices, vec![0]);
    assert!(state.arcs.is_empty());
    assert_eq!(patterns.len(), 2);
    assert!(patterns.iter().any(|pattern| {
        pattern.arcs
            == vec![
                StaticHelpArc {
                    helper: 0,
                    beneficiary: 1,
                },
                StaticHelpArc {
                    helper: 1,
                    beneficiary: 0,
                },
            ]
    }));
}

/// A supplied trail prefix is retained by every result and restored after recursive exploration.
///
/// @ai-generated
#[test]
fn enumerate_from_extends_and_restores_existing_prefix() {
    let prefix = StaticHelpArc {
        helper: 0,
        beneficiary: 1,
    };
    let graph = SearchGraph::new(&complete_help_arcs(&[0, 1, 2]));
    let component = HashSet::from([0, 1, 2]);
    let mut state = TrailState::new(&graph, 0);
    state.push(indexed_arc(&graph, prefix));
    let mut patterns = Vec::new();
    TrailSearch {
        graph: &graph,
        closed_component: &component,
        order: 3,
        max_len: 4,
        root: 0,
    }
    .enumerate_from(&mut state, &mut patterns);

    assert_eq!(state.vertices, vec![0, 1]);
    assert_eq!(state.arcs, vec![prefix]);
    assert!(!patterns.is_empty());
    assert!(patterns.iter().any(|pattern| {
        pattern.arcs
            == vec![
                prefix,
                StaticHelpArc {
                    helper: 1,
                    beneficiary: 2,
                },
                StaticHelpArc {
                    helper: 2,
                    beneficiary: 0,
                },
            ]
    }));
}

/// A prefix already at the edge bound is neither extended nor mutated.
///
/// @ai-generated
#[test]
fn enumerate_from_stops_at_maximum_length() {
    let prefix = StaticHelpArc {
        helper: 0,
        beneficiary: 1,
    };
    let graph = SearchGraph::new(&complete_help_arcs(&[0, 1]));
    let component = HashSet::from([0, 1]);
    let mut state = TrailState::new(&graph, 0);
    state.push(indexed_arc(&graph, prefix));
    let mut patterns = Vec::new();
    TrailSearch {
        graph: &graph,
        closed_component: &component,
        order: 2,
        max_len: 1,
        root: 0,
    }
    .enumerate_from(&mut state, &mut patterns);

    assert_eq!(state.vertices, vec![0, 1]);
    assert_eq!(state.arcs, vec![prefix]);
    assert!(patterns.is_empty());
}

/// Canonical search exactly matches the former every-root output on every graph up to order three.
///
/// @ai-generated
#[test]
fn canonical_search_matches_reference_on_all_small_digraphs() {
    for n_agents in 2usize..=3 {
        let arc_slots = n_agents * (n_agents - 1);
        for bits in 0..(1u64 << arc_slots) {
            let graph = graph_from_bits(n_agents, bits);
            for order in 2..=n_agents {
                assert_eq!(
                    enumerate_closed_trail_patterns(graph.clone(), order),
                    reference_patterns(graph.clone(), order),
                    "n_agents={n_agents}, bits={bits}, order={order}"
                );
            }
        }
    }
}

/// Canonical rooting preserves representative order-four bases, including supports omitting zero.
///
/// @ai-generated
#[test]
fn canonical_search_matches_reference_on_representative_order_four_graphs() {
    let graphs = [
        complete_help_arcs(&[0, 1, 2, 3]),
        vec![
            StaticHelpArc {
                helper: 0,
                beneficiary: 1,
            },
            StaticHelpArc {
                helper: 1,
                beneficiary: 0,
            },
            StaticHelpArc {
                helper: 1,
                beneficiary: 2,
            },
            StaticHelpArc {
                helper: 2,
                beneficiary: 1,
            },
        ],
        closed_word(&[0, 1, 2, 0, 1, 3, 0]).0,
    ];
    for graph in graphs {
        for order in 2..=4 {
            assert_eq!(
                enumerate_closed_trail_patterns(graph.clone(), order),
                reference_patterns(graph.clone(), order),
                "order={order}, graph={graph:?}"
            );
        }
    }
}

/// Higher-order canonical search matches the former output for sparse IDs and repeated structure.
///
/// @ai-generated
#[test]
fn canonical_search_matches_reference_for_non_contiguous_repeated_order_five() {
    let potential_arcs = closed_word(&[2, 7, 11, 2, 7, 19, 2, 7, 31, 2]).0;
    let reference = reference_patterns(potential_arcs.clone(), 5);
    let canonical = enumerate_closed_trail_patterns(potential_arcs, 5);

    assert!(!reference.is_empty());
    assert!(reference.iter().any(|pattern| {
        pattern
            .arcs
            .iter()
            .filter(|&&arc| {
                arc == StaticHelpArc {
                    helper: 2,
                    beneficiary: 7,
                }
            })
            .count()
            == 3
    }));
    assert_eq!(canonical, reference);
}

/// Input order, duplicates, and self-arcs do not change the exact sorted pattern vector.
///
/// @ai-generated
#[test]
fn pattern_output_is_deterministic_for_permuted_noisy_inputs() {
    let mut baseline_arcs = closed_word(&[3, 8, 13, 3, 8, 21, 3]).0;
    baseline_arcs.sort_unstable();
    baseline_arcs.dedup();
    let expected = enumerate_closed_trail_patterns(baseline_arcs.clone(), 4);
    assert!(!expected.is_empty());

    let mut reversed = baseline_arcs.clone();
    reversed.reverse();
    reversed.extend([reversed[0], reversed[2]]);
    reversed.extend([
        StaticHelpArc {
            helper: 3,
            beneficiary: 3,
        },
        StaticHelpArc {
            helper: 21,
            beneficiary: 21,
        },
    ]);

    let mut permuted = vec![
        baseline_arcs[3],
        baseline_arcs[0],
        baseline_arcs[4],
        baseline_arcs[2],
        baseline_arcs[1],
    ];
    permuted.extend([baseline_arcs[4], baseline_arcs[0]]);
    permuted.extend([
        StaticHelpArc {
            helper: 8,
            beneficiary: 8,
        },
        StaticHelpArc {
            helper: 13,
            beneficiary: 13,
        },
    ]);

    assert_eq!(enumerate_closed_trail_patterns(reversed, 4), expected);
    assert_eq!(enumerate_closed_trail_patterns(permuted, 4), expected);
}

/// Rotation regeneration recomputes the previous repeated-arc prefix across the new cut.
///
/// @ai-generated
#[test]
fn regenerated_rotations_recompute_repeated_arc_metadata() {
    let graph = closed_word(&[0, 1, 2, 0, 1, 3, 0]).0;
    let expected_arcs = vec![graph[1], graph[2], graph[3], graph[4], graph[5], graph[0]];
    let pattern = enumerate_closed_trail_patterns(graph, 4)
        .into_iter()
        .find(|pattern| pattern.arcs == expected_arcs)
        .expect("the shifted double-petal rotation must be regenerated");

    assert_eq!(
        pattern.previous_same_prefix_len,
        vec![None, None, None, None, None, Some(3)]
    );
}

/// Check the proven tight bounds used by the production enumerator.
///
/// @ai-generated
#[test]
fn irreducible_closed_trail_bound_is_tight_for_small_orders() {
    let expected = vec![
        None,
        None,
        Some(2),
        Some(4),
        Some(6),
        Some(9),
        Some(12),
        Some(16),
        Some(20),
        Some(25),
        Some(30),
    ];
    for (order, &bound) in expected.iter().enumerate() {
        assert_eq!(max_irreducible_closed_trail_len(order), bound);
    }
}

/// Enumerate the known four-agent rooted irreducible pattern basis deterministically.
///
/// @ai-generated
#[test]
fn four_agent_pattern_counts_match_the_irreducible_basis() {
    let complete_arcs = complete_help_arcs(&[0, 1, 2, 3]);
    assert_eq!(
        enumerate_closed_trail_patterns(complete_arcs.clone(), 2).len(),
        12
    );
    let order_three = enumerate_closed_trail_patterns(complete_arcs.clone(), 3);
    assert_eq!(
        order_three
            .iter()
            .filter(|pattern| pattern.arcs.len() == 3)
            .count(),
        24
    );
    assert_eq!(
        order_three
            .iter()
            .filter(|pattern| pattern.arcs.len() == 4)
            .count(),
        48
    );
    assert_eq!(order_three.len(), 72);
    assert_eq!(
        enumerate_closed_trail_patterns(complete_arcs.clone(), 4).len(),
        336
    );
    assert_eq!(
        enumerate_closed_trail_patterns(complete_arcs.clone(), 4),
        enumerate_closed_trail_patterns(complete_arcs, 4)
    );
}
