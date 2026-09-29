use crate::AgentId;

use super::interdependence::StaticHelpArc;

/// A static directed sequence pattern and the earlier prefixes needed to separate repeated arcs.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SequencePattern {
    pub arcs: Vec<StaticHelpArc>,
    pub previous_same_prefix_len: Vec<Option<usize>>,
}

/// Enumerate every static directed sequence pattern of exactly `length` arcs.
///
/// Only laser-owning agents can occur as helpers. The final beneficiary may be any agent because it
/// does not need to provide a subsequent help event. Pattern order is deterministic because indices
/// are embedded in SAT progress-variable keys.
pub fn enumerate_sequence_patterns(
    mut helper_ids: Vec<AgentId>,
    n_agents: usize,
    length: usize,
) -> Vec<SequencePattern> {
    if length == 0 {
        return Vec::new();
    }
    helper_ids.sort_unstable();
    helper_ids.dedup();

    let mut patterns = Vec::new();
    for &root in &helper_ids {
        let mut arcs = Vec::with_capacity(length);
        enumerate_from(
            &helper_ids,
            n_agents,
            length,
            root,
            &mut arcs,
            &mut patterns,
        );
    }
    patterns
}

/// Extend one static sequence prefix until it contains exactly `length` arcs.
fn enumerate_from(
    helper_ids: &[AgentId],
    n_agents: usize,
    length: usize,
    current: AgentId,
    arcs: &mut Vec<StaticHelpArc>,
    patterns: &mut Vec<SequencePattern>,
) {
    let completes_pattern = arcs.len() + 1 == length;
    for beneficiary in 0..n_agents {
        if beneficiary == current
            || (!completes_pattern && helper_ids.binary_search(&beneficiary).is_err())
        {
            continue;
        }

        arcs.push(StaticHelpArc {
            helper: current,
            beneficiary,
        });
        if completes_pattern {
            patterns.push(SequencePattern {
                previous_same_prefix_len: previous_same_prefix_lens(arcs),
                arcs: arcs.clone(),
            });
        } else {
            enumerate_from(helper_ids, n_agents, length, beneficiary, arcs, patterns);
        }
        arcs.pop();
    }
}

/// Locate the most recent preceding occurrence of each static arc.
///
/// Repeated static arcs must occur at strictly increasing timestamps so they do not reuse one
/// temporal help edge.
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
