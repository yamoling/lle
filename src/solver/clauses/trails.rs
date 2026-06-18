//! Pure directed-graph enumeration for the cooperation-forbid modes.
//!
//! These functions have no dependency on the SAT machinery (`ClauseGenerator`, `VarPool`,
//! `World`); they operate purely over agent ids. [`enumerate_for_mode`] is the single entry point
//! used by the generator to turn a [`SolveMode`] into the set of directed trails it must forbid.

use std::collections::HashSet;

use super::solve_mode::SolveMode;
use crate::AgentId;

/// The directed trails to forbid for `mode`, or an empty vector for modes that do not use the
/// trail machinery.
///
/// - [`SolveMode::NoInterdependence`] yields simple directed cycles of order ≥ `n`, each expanded
///   to all of its [`closed_rotations`] so the forbid is independent of which edge fires first.
/// - [`SolveMode::NoChainedCooperation`] yields all directed trails of exactly `k` edges;
///   forbidding each one prevents chains ≥ `k` because every longer trail contains a sub-trail of
///   length `k`.
pub(crate) fn enumerate_for_mode(
    mode: SolveMode,
    owners: &[AgentId],
    all_agents: &[AgentId],
) -> Vec<Vec<AgentId>> {
    match mode {
        SolveMode::NoInterdependence(n) => enumerate_directed_cycles(owners, n)
            .into_iter()
            .flat_map(|cycle| closed_rotations(&cycle))
            .collect(),
        SolveMode::NoChainedCooperation(k) => enumerate_directed_trails(owners, all_agents, k),
        _ => vec![],
    }
}

/// Every rotation of a directed `cycle`, each returned as a closed vertex sequence
/// `[v_i, …, v_i]`.
///
/// [`enumerate_directed_cycles`] returns each directed cycle once, linearized from its
/// lexicographically-smallest agent. But a *temporal* cycle is realized by
/// [`ClauseGenerator::trail_clauses`] only when its edges fire in the linearized order with
/// non-decreasing timestamps, so the realizable linearization must start at whichever edge
/// happens earliest in time. Forbidding only the canonical rotation misses cycles whose earliest
/// help event does not originate from the smallest-id agent. Emitting all rotations (each a
/// separate forbidden trail) makes the forbid independent of which edge fires first.
///
/// [`ClauseGenerator::trail_clauses`]: super::generator::ClauseGenerator::trail_clauses
fn closed_rotations(cycle: &[AgentId]) -> Vec<Vec<AgentId>> {
    (0..cycle.len())
        .map(|start| {
            let mut rotated: Vec<AgentId> = cycle[start..]
                .iter()
                .chain(&cycle[..start])
                .copied()
                .collect();
            rotated.push(rotated[0]);
            rotated
        })
        .collect()
}

/// Enumerate all simple directed cycles of order ≥ `min_order` over `agents`.
///
/// Each cycle is returned as a Vec whose first element is the lexicographically-smallest agent
/// in the cycle (canonical form that avoids counting the same cycle under different rotations).
/// Callers that need every temporal linearization expand each cycle with [`closed_rotations`].
fn enumerate_directed_cycles(agents: &[AgentId], min_order: usize) -> Vec<Vec<AgentId>> {
    let mut cycles = Vec::new();
    for (start_idx, &start) in agents.iter().enumerate() {
        let available: Vec<AgentId> = agents[start_idx + 1..].to_vec();
        cycles_dfs(&available, vec![start], min_order, &mut cycles);
    }
    cycles
}

fn cycles_dfs(
    available: &[AgentId],
    path: Vec<AgentId>,
    min_order: usize,
    out: &mut Vec<Vec<AgentId>>,
) {
    if path.len() >= min_order {
        out.push(path.clone());
    }
    for (i, &next) in available.iter().enumerate() {
        let mut new_avail = available.to_vec();
        new_avail.remove(i);
        let mut new_path = path.clone();
        new_path.push(next);
        cycles_dfs(&new_avail, new_path, min_order, out);
    }
}

/// Enumerate all directed trails of exactly `length` edges starting from a laser owner.
///
/// Each trail is returned as `[v0, v1, …, v_length]` where:
/// - `v0, …, v_{length-1}` must be laser owners (they are helpers);
/// - `v_length` can be any agent (the final beneficiary);
/// - no directed pair `(vi, v_{i+1})` appears twice (trail condition);
/// - no self-loops (`vi ≠ v_{i+1}`).
fn enumerate_directed_trails(
    owners: &[AgentId],
    all_agents: &[AgentId],
    length: usize,
) -> Vec<Vec<AgentId>> {
    if length < 2 {
        return vec![];
    }
    let mut out = Vec::new();
    let mut edges_used = HashSet::new();
    for &start in owners {
        let mut path = vec![start];
        trail_dfs(
            start,
            owners,
            all_agents,
            &mut edges_used,
            &mut path,
            length,
            &mut out,
        );
    }
    out
}

fn trail_dfs(
    current: AgentId,
    owners: &[AgentId],
    all_agents: &[AgentId],
    edges_used: &mut HashSet<(AgentId, AgentId)>,
    path: &mut Vec<AgentId>,
    target_edges: usize,
    out: &mut Vec<Vec<AgentId>>,
) {
    let current_edges = path.len() - 1;
    if current_edges == target_edges {
        out.push(path.clone());
        return;
    }
    let remaining = target_edges - current_edges;
    // All helpers except the last must be laser owners; the final beneficiary can be any agent.
    let candidates: &[AgentId] = if remaining == 1 { all_agents } else { owners };
    for &next in candidates {
        if next == current {
            continue;
        }
        let edge = (current, next);
        if !edges_used.contains(&edge) {
            edges_used.insert(edge);
            path.push(next);
            trail_dfs(
                next,
                owners,
                all_agents,
                edges_used,
                path,
                target_edges,
                out,
            );
            path.pop();
            edges_used.remove(&edge);
        }
    }
}
