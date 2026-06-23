use std::collections::HashSet;

use super::position_set::PositionSet;
use crate::{AgentId, World, tiles::LaserId};

/// A potential-help edge at one time step. An `PotentialHelpEdge`
/// indicates that it is possible for agent `helper` to help agent
/// `beneficiary` at time step `t`.
///
/// It **does not** indicate that the agent **is** helped, only that
/// it could happen.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PotentialHelpEdge {
    pub helper: AgentId,
    pub beneficiary: AgentId,
    pub t: usize,
}

/// Dense directed adjacency matrix over the agent set.
/// Helper agents (i.e. nodes from which the edge originates) can
/// only be agents that have a laser of the same colour as theirs.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AdjacencyMatrix {
    n_agents: usize,
    edges: Vec<bool>,
}

impl AdjacencyMatrix {
    pub fn new(n_agents: usize) -> Self {
        Self {
            n_agents,
            edges: vec![false; n_agents * n_agents],
        }
    }

    #[inline]
    pub fn n_agents(&self) -> usize {
        self.n_agents
    }

    #[inline]
    fn idx(&self, helper: AgentId, beneficiary: AgentId) -> usize {
        helper * self.n_agents + beneficiary
    }

    pub fn insert(&mut self, helper: AgentId, beneficiary: AgentId) {
        if helper != beneficiary {
            let idx = self.idx(helper, beneficiary);
            self.edges[idx] = true;
        }
    }

    pub fn contains(&self, helper: AgentId, beneficiary: AgentId) -> bool {
        if helper == beneficiary {
            return false;
        }
        self.edges[self.idx(helper, beneficiary)]
    }

    pub fn outgoing_from(&self, helper: AgentId) -> Vec<AgentId> {
        (0..self.n_agents)
            .filter(|&beneficiary| self.contains(helper, beneficiary))
            .collect()
    }

    pub fn incoming_to(&self, beneficiary: AgentId) -> Vec<AgentId> {
        (0..self.n_agents)
            .filter(|&helper| self.contains(helper, beneficiary))
            .collect()
    }

    pub fn edges(&self) -> Vec<(AgentId, AgentId)> {
        let mut edges = Vec::new();
        for helper in 0..self.n_agents {
            for beneficiary in 0..self.n_agents {
                if self.contains(helper, beneficiary) {
                    edges.push((helper, beneficiary));
                }
            }
        }
        edges
    }

    pub fn is_empty(&self) -> bool {
        self.edges.iter().all(|edge| !edge)
    }
}

/// Temporal graph of all potential cooperation relationships within one bounded horizon.
///
/// `adjacency_matrices[t]` stores the directed potential-help graph at time step `t`.
#[derive(Clone, Debug)]
pub struct PotentialCooperationGraph {
    n_agents: usize,
    adjacency_matrices: Vec<AdjacencyMatrix>,
    laser_colours: Vec<(LaserId, AgentId)>,
}

impl PotentialCooperationGraph {
    /// Build an empty potential-cooperation graph for a bounded horizon.
    ///
    /// Laser ownership is stored by laser id so later updates can align owners with the
    /// context-computed `relevant_lasers[laser_id]` cache.
    pub fn new(world: &World, t_max: usize) -> Self {
        let n_agents = world.n_agents();
        let mut laser_colours: Vec<(usize, AgentId)> = world
            .sources()
            .map(|(_, source)| (source.laser_id(), source.agent_id()))
            .collect();
        laser_colours.sort_by_key(|(laser_id, _)| *laser_id);
        // let laser_colours = laser_colours
        //     .into_iter()
        //     .map(|(_, agent_id)| agent_id)
        //     .collect();
        Self {
            n_agents,
            adjacency_matrices: (0..=t_max)
                .map(|_| AdjacencyMatrix::new(n_agents))
                .collect(),
            laser_colours,
        }
    }

    pub fn from_adjacency_matrices(matrices: Vec<AdjacencyMatrix>) -> Self {
        Self {
            n_agents: matrices[0].n_agents,
            adjacency_matrices: matrices,
            laser_colours: Vec::new(),
        }
    }

    /// Compute and store the adjacency matrix for a single time step.
    ///
    /// Arguments:
    ///   - `relevant_positions`: `relevant_positions[agent][t]` contains the positions relevant to
    ///     an agent at `t`.
    ///   - `relevant_lasers`: `relevant_lasers[laser_id][t]` contains the laser tiles relevant to
    ///     cooperation at `t`.
    pub fn update(
        &mut self,
        t: usize,
        relevant_positions: &[Vec<PositionSet>],
        relevant_lasers: &[Vec<PositionSet>],
    ) {
        let mut adjacency = AdjacencyMatrix::new(self.n_agents);
        for &(laser_id, helper) in &self.laser_colours {
            let laser_tiles = &relevant_lasers[laser_id][t];
            if laser_tiles.is_empty() {
                continue;
            }
            for (beneficiary, positions) in relevant_positions.iter().enumerate() {
                if beneficiary != helper && laser_tiles.intersection(&positions[t]).next().is_some()
                {
                    adjacency.insert(helper, beneficiary);
                }
            }
        }
        self.adjacency_matrices[t] = adjacency;
    }

    pub fn n_agents(&self) -> usize {
        self.n_agents
    }

    pub fn horizon(&self) -> usize {
        self.adjacency_matrices.len().saturating_sub(1)
    }

    pub fn at(&self, t: usize) -> &AdjacencyMatrix {
        &self.adjacency_matrices[t]
    }

    pub fn has_edge(&self, helper: AgentId, beneficiary: AgentId, t: usize) -> bool {
        self.at(t).contains(helper, beneficiary)
    }

    pub fn edges_at(&self, t: usize) -> Vec<PotentialHelpEdge> {
        self.adjacency_matrices[t]
            .edges()
            .into_iter()
            .map(|(helper, beneficiary)| PotentialHelpEdge {
                helper,
                beneficiary,
                t,
            })
            .collect()
    }

    /// Enumerate all temporal directed trails of exactly `length` edges with non-decreasing times.
    ///
    /// Temporal edges are triples `(helper, beneficiary, t)`. A trail may revisit agents freely, but
    /// it never repeats the same temporal edge.
    pub fn enumerate_trails_of_length(&self, length: usize) -> Vec<Vec<PotentialHelpEdge>> {
        if length == 0 {
            return Vec::new();
        }
        let mut out = Vec::new();
        for start_t in 0..=self.horizon() {
            for (helper, beneficiary) in self.at(start_t).edges() {
                let start = PotentialHelpEdge {
                    helper,
                    beneficiary,
                    t: start_t,
                };
                let mut used = HashSet::from([start]);
                let mut path = vec![start];
                self.enumerate_trails(beneficiary, start_t, length, &mut used, &mut path, &mut out);
            }
        }
        out
    }

    fn enumerate_trails(
        &self,
        current: AgentId,
        last_t: usize,
        target_len: usize,
        used: &mut HashSet<PotentialHelpEdge>,
        path: &mut Vec<PotentialHelpEdge>,
        out: &mut Vec<Vec<PotentialHelpEdge>>,
    ) {
        if path.len() == target_len {
            out.push(path.clone());
            return;
        }
        for t in last_t..=self.horizon() {
            for next in self.at(t).outgoing_from(current) {
                let edge = PotentialHelpEdge {
                    helper: current,
                    beneficiary: next,
                    t,
                };
                if used.insert(edge) {
                    path.push(edge);
                    self.enumerate_trails(next, t, target_len, used, path, out);
                    path.pop();
                    used.remove(&edge);
                }
            }
        }
    }

    /// Enumerate all temporal simple directed cycles of exactly `length` edges with non-decreasing
    /// times.
    ///
    /// The cycle order equals the number of distinct agents on the cycle, so every non-closing
    /// vertex must be unique and the last edge must return to the start agent.
    pub fn enumerate_cycles_of_length(&self, length: usize) -> Vec<Vec<PotentialHelpEdge>> {
        if length < 2 {
            return Vec::new();
        }
        let mut out = Vec::new();
        for start_t in 0..=self.horizon() {
            for (helper, beneficiary) in self.at(start_t).edges() {
                let start = PotentialHelpEdge {
                    helper,
                    beneficiary,
                    t: start_t,
                };
                let mut used_edges = HashSet::from([start]);
                let mut visited_agents = HashSet::from([helper, beneficiary]);
                let mut path = vec![start];
                self.cycles_dfs(
                    helper,
                    beneficiary,
                    start_t,
                    length,
                    &mut used_edges,
                    &mut visited_agents,
                    &mut path,
                    &mut out,
                );
            }
        }
        out
    }

    fn cycles_dfs(
        &self,
        start: AgentId,
        current: AgentId,
        last_t: usize,
        target_len: usize,
        used_edges: &mut HashSet<PotentialHelpEdge>,
        visited_agents: &mut HashSet<AgentId>,
        path: &mut Vec<PotentialHelpEdge>,
        out: &mut Vec<Vec<PotentialHelpEdge>>,
    ) {
        if path.len() == target_len {
            if current == start {
                out.push(path.clone());
            }
            return;
        }
        let remaining = target_len - path.len();
        for t in last_t..=self.horizon() {
            for next in self.at(t).outgoing_from(current) {
                let edge = PotentialHelpEdge {
                    helper: current,
                    beneficiary: next,
                    t,
                };
                if used_edges.contains(&edge) {
                    continue;
                }
                if remaining == 1 {
                    if next != start {
                        continue;
                    }
                } else if next == start || visited_agents.contains(&next) {
                    continue;
                }

                used_edges.insert(edge);
                path.push(edge);
                let inserted = next != start && visited_agents.insert(next);
                self.cycles_dfs(
                    start,
                    next,
                    t,
                    target_len,
                    used_edges,
                    visited_agents,
                    path,
                    out,
                );
                if inserted {
                    visited_agents.remove(&next);
                }
                path.pop();
                used_edges.remove(&edge);
            }
        }
    }
}

#[cfg(test)]
#[path = "../unit_tests/test_potential_cooperation.rs"]
mod test;
