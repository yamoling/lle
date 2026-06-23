use std::collections::HashSet;

use rstest::rstest;

use crate::{AgentId, World};

use super::super::context::ConstraintContext;
use super::{AdjacencyMatrix, PotentialCooperationGraph, PotentialHelpEdge};

/// Build a `PotentialCooperationGraph` from an explicit list of directed edges per time step.
/// `edges_per_t[t]` holds the `(helper, beneficiary)` pairs present at step `t`.
fn graph_from(n_agents: usize, edges_per_t: &[&[(AgentId, AgentId)]]) -> PotentialCooperationGraph {
    let matrices = edges_per_t
        .iter()
        .map(|edges| {
            let mut m = AdjacencyMatrix::new(n_agents);
            for &(helper, beneficiary) in *edges {
                m.insert(helper, beneficiary);
            }
            m
        })
        .collect();
    PotentialCooperationGraph::from_adjacency_matrices(matrices)
}

#[test]
fn adjacency_matrix_edges_no_duplicates() {
    let mut m = AdjacencyMatrix::new(4);
    assert_eq!(m.edges().len(), 0);
    m.insert(0, 1);
    assert_eq!(m.edges().len(), 1);
    assert_eq!(m.edges()[0], (0, 1));
    m.insert(0, 1);
    assert_eq!(m.edges().len(), 1);
}

#[test]
fn adjacency_matrix_edges_contains() {
    let mut m = AdjacencyMatrix::new(4);
    assert_eq!(m.edges().len(), 0);
    m.insert(0, 1);
    assert!(m.contains(0, 1));
    assert!(!m.contains(1, 0));
}

#[test]
fn adjacency_matrix_ignores_self_loops_and_tracks_neighbours() {
    let mut m = AdjacencyMatrix::new(3);
    assert!(m.is_empty());

    m.insert(0, 0);
    assert!(!m.contains(0, 0));
    assert!(m.is_empty());

    m.insert(0, 1);
    m.insert(2, 1);

    assert_eq!(m.outgoing_from(0), vec![1]);
    assert_eq!(m.outgoing_from(1), Vec::<usize>::new());
    assert_eq!(m.incoming_to(1), vec![0, 2]);
    assert_eq!(m.incoming_to(0), Vec::<usize>::new());
    assert_eq!(m.edges(), vec![(0, 1), (2, 1)]);
    assert!(!m.is_empty());
}

#[test]
fn potential_cooperation_graph_from_adjacency_matrices_exposes_edges() {
    let mut t0 = AdjacencyMatrix::new(3);
    let mut t1 = AdjacencyMatrix::new(3);
    t0.insert(1, 2);
    t1.insert(0, 1);
    t1.insert(2, 1);

    let graph = PotentialCooperationGraph::from_adjacency_matrices(vec![t0, t1]);
    assert_eq!(graph.n_agents(), 3);
    assert_eq!(graph.horizon(), 1);
    assert!(graph.has_edge(1, 2, 0));
    assert!(graph.has_edge(0, 1, 1));
    assert!(graph.has_edge(2, 1, 1));
    assert!(!graph.has_edge(1, 0, 1));
    assert_eq!(
        graph.edges_at(1),
        vec![
            PotentialHelpEdge {
                helper: 0,
                beneficiary: 1,
                t: 1,
            },
            PotentialHelpEdge {
                helper: 2,
                beneficiary: 1,
                t: 1,
            },
        ]
    );
}

#[rstest]
#[case(World::try_from("S0 X").unwrap())]
#[case(World::get_level(1).unwrap())]
fn cooperation_graph_empty_with_one_agent(#[case] world: World) {
    let mut ctx = ConstraintContext::new(&world, 5);
    for t in 0..=ctx.t_max {
        ctx.update(t);
        assert_eq!(ctx.potential_cooperation.edges_at(t).len(), 0);
    }
}

#[test]
fn potential_graph_two_agents() {
    let world = World::try_from(
        "
        @  S0 S1  @
       L0E .  .   @
        @  .  .  L1W
        @  X  X   @
        ",
    )
    .unwrap();
    let mut ctx = ConstraintContext::new(&world, 10);
    ctx.update(10);
    let graph = ctx.potential_cooperation;
    eprintln!("{graph:?}");
    assert_eq!(graph.edges_at(0).len(), 0);
    assert_eq!(graph.edges_at(1).len(), 1);
    for t in 2..=8 {
        assert_eq!(graph.edges_at(t).len(), 2);
    }
    assert_eq!(graph.edges_at(9).len(), 1);
    assert_eq!(graph.edges_at(10).len(), 0);
}

/// `update` records a one-way help edge from the laser owner to a non-owner that can stand
/// downstream of the blocked beam, and never records the reverse (a non-owner cannot help).
#[test]
fn update_records_one_way_help_edge() {
    // Agent 0 owns the east-facing beam on row 1; agent 1 must cross it (tiles (1,1)/(1,2)) to
    // reach its exit, so agent 0 can help agent 1 but not vice-versa (agent 1 owns no laser).
    let t_max = 8;
    let world = World::try_from(
        " S0 . S1
         L0E . .
          X  . X",
    )
    .expect("failed to parse world");
    let mut ctx = ConstraintContext::new(&world, t_max);
    ctx.update(t_max);
    let graph = ctx.potential_cooperation;
    // There should never be a help edge starting from agent 1 since there is no laser of its colour
    for t in 0..=t_max {
        assert!(
            !graph.has_edge(1, 0, t),
            "a non-owner can never help, t={t}"
        );
    }
    // Starting from t=2, and up to t_max-2, there should be a potential help edge
    // between 0 and 1.
    assert!(!graph.has_edge(0, 1, 0));
    assert!(!graph.has_edge(0, 1, 1));
    for t in 2..=(t_max - 2) {
        assert!(graph.has_edge(0, 1, t),);
    }
    assert!(!graph.has_edge(0, 1, t_max - 1));
    assert!(!graph.has_edge(0, 1, t_max));
}

/// `horizon` is one less than the number of stored matrices, and `at`/`has_edge` agree with the
/// matrices the graph was built from.
#[test]
fn horizon_and_at_reflect_the_stored_matrices() {
    let graph = graph_from(3, &[&[(0, 1)], &[(1, 2)], &[]]);
    assert_eq!(graph.horizon(), 2);
    assert_eq!(graph.at(0).n_agents(), 3);
    assert!(graph.at(0).contains(0, 1));
    assert!(graph.has_edge(0, 1, 0));
    assert!(!graph.has_edge(0, 1, 1));
    assert!(graph.at(1).contains(1, 2));
    assert!(graph.has_edge(1, 2, 1));
    assert!(graph.at(2).is_empty());
}

/// Length 0 yields no trail; length 1 yields exactly one trail per temporal edge.
#[test]
fn enumerate_trails_length_0_and_1() {
    let graph = graph_from(3, &[&[(0, 1)], &[(1, 2)]]);
    assert!(graph.enumerate_trails_of_length(0).is_empty());
    let singles = graph.enumerate_trails_of_length(1);
    assert_eq!(singles.len(), 2);
    assert!(singles.iter().all(|trail| trail.len() == 1));
}

/// A length-2 trail chains two edges with non-decreasing time across consecutive steps.
#[test]
fn enumerate_trails_across_time() {
    let graph = graph_from(3, &[&[(0, 1)], &[(1, 2)]]);
    let trails = graph.enumerate_trails_of_length(2);
    assert_eq!(
        trails,
        vec![vec![
            PotentialHelpEdge {
                helper: 0,
                beneficiary: 1,
                t: 0
            },
            PotentialHelpEdge {
                helper: 1,
                beneficiary: 2,
                t: 1
            },
        ]]
    );
}

/// Edges cannot be chained backwards in time: an edge at `t=1` may not be followed by one at
/// `t=0`, even when the agents would otherwise connect.
#[test]
fn enumerate_trails_respects_non_decreasing_time() {
    // (1->2) at t=0 and (0->1) at t=1: chaining (0->1) into (1->2) would require going back in
    // time, so no length-2 trail exists.
    let graph = graph_from(3, &[&[(1, 2)], &[(0, 1)]]);
    assert!(graph.enumerate_trails_of_length(2).is_empty());
}

/// Two opposite edges in the same time step form two length-2 trails, but no length-3 trail
/// exists because each temporal edge may be used at most once.
#[test]
fn enumerate_trails_same_step_forbids_edge_reuse() {
    let graph = graph_from(2, &[&[(0, 1), (1, 0)]]);
    assert_eq!(graph.enumerate_trails_of_length(2).len(), 2);
    assert!(graph.enumerate_trails_of_length(3).is_empty());
}

#[test]
fn enumerate_trails_same_step_plus_one_is_length_three() {
    let graph = graph_from(3, &[&[(0, 1), (1, 0)], &[(1, 2)]]);
    // 0->1->0, 1->0->1, 0->1->2
    assert_eq!(graph.enumerate_trails_of_length(2).len(), 3);
    // 1->0->1->2
    let trails3 = graph.enumerate_trails_of_length(3);
    assert_eq!(trails3.len(), 1);
    assert_eq!(trails3[0].len(), 3);
    assert_eq!(
        trails3[0][0],
        PotentialHelpEdge {
            helper: 1,
            beneficiary: 0,
            t: 0
        }
    );
    assert_eq!(
        trails3[0][1],
        PotentialHelpEdge {
            helper: 0,
            beneficiary: 1,
            t: 0
        }
    );
    assert_eq!(
        trails3[0][2],
        PotentialHelpEdge {
            helper: 1,
            beneficiary: 2,
            t: 1
        }
    );
}

/// A trail may revisit agents as long as it never reuses the same temporal edge.
#[test]
fn enumerate_trails_may_revisit_agents() {
    // 0->1 (t0), 1->0 (t1), 0->1 (t2): a length-3 trail that revisits agents 0 and 1.
    let graph = graph_from(2, &[&[(0, 1)], &[(1, 0)], &[(0, 1)]]);
    let trails = graph.enumerate_trails_of_length(3);
    assert_eq!(trails.len(), 1);
    assert_eq!(
        trails[0],
        vec![
            PotentialHelpEdge {
                helper: 0,
                beneficiary: 1,
                t: 0
            },
            PotentialHelpEdge {
                helper: 1,
                beneficiary: 0,
                t: 1
            },
            PotentialHelpEdge {
                helper: 0,
                beneficiary: 1,
                t: 2
            },
        ]
    );
}

/// A cycle needs at least two edges; shorter lengths are rejected, even though cycles
/// exist in the provided graph.
#[test]
fn enumerate_cycles_below_2_is_empty() {
    let graph = graph_from(2, &[&[(0, 1), (1, 0)]]);
    assert!(graph.enumerate_cycles_of_length(0).is_empty());
    assert!(graph.enumerate_cycles_of_length(1).is_empty());
}

/// A mutual `a -> b` then `b -> a` across two steps is a single order-2 temporal cycle that
/// returns to its start agent.
#[test]
fn enumerate_cycles_two_agents() {
    let graph = graph_from(2, &[&[(0, 1)], &[(1, 0)]]);
    let cycles = graph.enumerate_cycles_of_length(2);
    assert_eq!(cycles.len(), 1);
    let cycle = &cycles[0];
    assert_eq!(cycle.len(), 2);
    assert_eq!(
        cycle,
        &vec![
            PotentialHelpEdge {
                helper: 0,
                beneficiary: 1,
                t: 0
            },
            PotentialHelpEdge {
                helper: 1,
                beneficiary: 0,
                t: 1
            }
        ]
    );
}

/// A 3-agent simultaneous ring `a -> b -> c -> a` is an order-3 cycle.
/// Each of its edges is a valid start, so the enumeration returns the three rotations,
/// each visiting three distinct agents and closing back to its start.
#[test]
fn enumerate_cycles_three_agents() {
    let graph = graph_from(3, &[&[(0, 1), (1, 2), (2, 0)]]);
    let cycles = graph.enumerate_cycles_of_length(3);
    assert_eq!(cycles.len(), 3);
    for cycle in &cycles {
        assert_eq!(cycle.len(), 3);
        assert_eq!(cycle[0].helper, cycle[2].beneficiary);
        let agents: HashSet<AgentId> = cycle.iter().map(|edge| edge.helper).collect();
        assert_eq!(
            agents.len(),
            3,
            "an order-3 cycle visits three distinct agents"
        );
    }
}

/// Cycle enumeration filters by exact order: a graph containing only a 2-cycle has no 3-cycle.
#[test]
fn enumerate_cycles_filters_by_exact_length() {
    let graph = graph_from(3, &[&[(0, 1)], &[(1, 0)]]);
    assert_eq!(graph.enumerate_cycles_of_length(2).len(), 1);
    assert!(graph.enumerate_cycles_of_length(3).is_empty());
}

/// A purely acyclic temporal graph yields no cycles of any order.
#[test]
fn enumerate_cycles_acyclic_graph_is_empty() {
    let graph = graph_from(3, &[&[(0, 1)], &[(1, 2)]]);
    assert!(graph.enumerate_cycles_of_length(2).is_empty());
    assert!(graph.enumerate_cycles_of_length(3).is_empty());
}
