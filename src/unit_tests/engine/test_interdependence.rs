use super::{
    StaticHelpArc, enumerate_closed_trail_patterns, enumerate_from, is_splice_reducible,
    max_irreducible_closed_trail_len, previous_same_prefix_lens,
};
use crate::AgentId;
use rstest::rstest;

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
    assert!(enumerate_closed_trail_patterns([0, 1, 2, 3].into(), 5).is_empty());
}

/// Pattern enumeration uses only helper IDs and normalizes their order and duplicates.
///
/// @ai-generated
#[test]
fn enumerate_closed_trails_excludes_non_helpers() {
    let patterns = enumerate_closed_trail_patterns([3, 1, 3].into(), 2);

    assert_eq!(patterns, enumerate_closed_trail_patterns([1, 3].into(), 2));
    assert_eq!(patterns.len(), 2);
    assert!(
        patterns
            .iter()
            .flat_map(|pattern| &pattern.arcs)
            .all(|arc| { [1, 3].contains(&arc.helper) && [1, 3].contains(&arc.beneficiary) })
    );
}

/// Starting from the root enumerates the only possible order-two closed trail and restores state.
///
/// @ai-generated
#[test]
fn enumerate_from_emits_closed_trail_and_restores_root_prefix() {
    let mut vertices = vec![0];
    let mut arcs = Vec::new();
    let mut patterns = Vec::new();

    enumerate_from(&[0, 1], 2, 2, 0, &mut vertices, &mut arcs, &mut patterns);
    assert_eq!(patterns.len(), 1);
    assert_eq!(
        patterns[0].arcs,
        vec![
            StaticHelpArc {
                helper: 0,
                beneficiary: 1,
            },
            StaticHelpArc {
                helper: 1,
                beneficiary: 0,
            },
        ]
    );
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
    let mut vertices = vec![0, 1];
    let mut arcs = vec![prefix];
    let mut patterns = Vec::new();

    enumerate_from(&[0, 1, 2], 3, 4, 0, &mut vertices, &mut arcs, &mut patterns);

    assert_eq!(vertices, vec![0, 1]);
    assert_eq!(arcs, vec![prefix]);
    assert!(!patterns.is_empty());
    assert!(patterns.iter().all(|pattern| pattern.arcs[0] == prefix));
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
    let mut vertices = vec![0, 1];
    let mut arcs = vec![prefix];
    let mut patterns = Vec::new();

    enumerate_from(&[0, 1], 2, 1, 0, &mut vertices, &mut arcs, &mut patterns);

    assert_eq!(vertices, vec![0, 1]);
    assert_eq!(arcs, vec![prefix]);
    assert!(patterns.is_empty());
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
    let helper_ids = vec![0, 1, 2, 3];
    assert_eq!(
        enumerate_closed_trail_patterns(helper_ids.clone(), 2).len(),
        12
    );
    let order_three = enumerate_closed_trail_patterns(helper_ids.clone(), 3);
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
        enumerate_closed_trail_patterns(helper_ids.clone(), 4).len(),
        336
    );
    assert_eq!(
        enumerate_closed_trail_patterns(helper_ids.clone(), 4),
        enumerate_closed_trail_patterns(helper_ids.clone(), 4)
    );
}
