//! Pure directed-graph enumeration for the cooperation-forbid modes.
//!
//! These functions have no dependency on the SAT machinery (`ClauseGenerator`, `VarPool`,
//! `World`); they operate purely over agent ids. They enumerate the two explicit dependency
//! shapes consumed by mode support: open chains and closed cycle rotations.

use std::collections::HashSet;

use crate::AgentId;

/// Enumerate all directed paths of exactly `length` help edges starting from a laser owner.
/// A path is a walk in which vertices/nodes do not repeat. Since no node repeats, it cannot
/// contain cycles.
///
/// Each path is returned as `[v0, v1, …, v_length]` where:
/// - `v0, …, v_{length-1}` must be laser owners (they are helpers);
/// - `v_length` can be any agent (the final beneficiary);
/// - no directed pair `(vi, v_{i+1})` appears twice;
/// - no self-loops (`vi ≠ v_{i+1}`).
///
/// Since only laser owners can have outgoing help edges, a non-owner can only appear as the final
/// beneficiary. If such an agent is reached earlier, the partial path is not a valid chain of the
/// requested length and is therefore not emitted.
pub(crate) fn enumerate_paths(
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
        path_dfs(
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

/// Enumerate all temporal linearizations of simple directed cycles of order ≥ `min_order` over
/// laser owners.
///
/// The returned values are closed vertex sequences `[v_i, …, v_i]`. Each simple directed cycle is
/// expanded to all closed rotations so forbidding temporal cycles is independent of which edge fires
/// first in a concrete trajectory.
pub(crate) fn enumerate_cycles(owners: &[AgentId], min_order: usize) -> Vec<Vec<AgentId>> {
    enumerate_directed_cycles(owners, min_order)
        .into_iter()
        .flat_map(|cycle| closed_rotations(&cycle))
        .collect()
}

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
/// Each cycle is returned as a Vec whose first element is the lexicographically-smallest agent in
/// the cycle (canonical form that avoids counting the same cycle under different rotations).
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

fn path_dfs(
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
            path_dfs(
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

#[cfg(test)]
#[path = "../../unit_tests/test_dependency_shapes.rs"]
mod tests;
