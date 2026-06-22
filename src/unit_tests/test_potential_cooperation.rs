use rstest::rstest;

use crate::World;

use super::super::context::ConstraintContext;
use super::{AdjacencyMatrix, PotentialCooperationGraph, PotentialHelpEdge};

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
