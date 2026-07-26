use std::collections::HashSet;

use rstest::rstest;

use crate::{AgentId, World, solver::VarKey};

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

/// Converts potential-help edges into their matching SAT variable keys.
#[test]
fn potential_help_edge_converts_to_help_var_key() {
    let edge = PotentialHelpEdge {
        helper: 2,
        beneficiary: 1,
        t: 4,
    };

    let key: VarKey = edge.into();

    assert_eq!(
        key,
        VarKey::Help {
            helper: 2,
            beneficiary: 1,
            t: 4,
        }
    );
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

/// The four-agent chain layout must not expose `0 → 3` before agent 3 can reach a colour-0
/// beam tile. This prevents chain enumeration from considering physically impossible early edges.
#[test]
fn four_agent_chain_prunes_unreachable_early_help_edge() {
    let world = World::try_from(
        "
@   S0  S1 @   @
@   .   .  L1W @
L0E .   .  L1S @
@   X   .  .   @
@   L2S .  .   S2
@   X   .  @   @
S3  .   .  .   @
@   L3E .  .   @
@   @   .  .   L1W
@   @   X  X   @
",
    )
    .unwrap();
    let mut ctx = ConstraintContext::new(&world, 14);
    ctx.update(14);
    for t in 0..6 {
        assert!(
            !ctx.potential_cooperation.has_edge(0, 3, t),
            "agent 0 cannot help agent 3 at t={t}"
        );
    }
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

/// In this test, we enumerate all the possible help events that can occur within [0, t_max].
/// This test assumes some optimizations related to the accessibility of some tiles:
///     - the bottom left exit should only be accessible by agent 2 because its neighbouring tiles
///       are either only accessible to agent 2 (first laser beam) or an exit.
///     - time-wise reachability (from the start and to the exits)
#[test]
fn potential_graph_3agents() {
    let world = World::try_from(
        "
 @  S0  S1  S2
L0E  .   .  .
L1E  .   .  .
L2E  .   .  .
 @   X   X  X
",
    )
    .unwrap();

    fn verify_edges(t: usize, ctx: &mut ConstraintContext, edges: &Vec<(usize, usize)>) {
        eprintln!("\nt={t}");
        ctx.update(t);
        for agent in 0..ctx.n_agents {
            let positions = ctx.relevant_positions_for_agent(agent, t);
            eprintln!("\tAgent {agent} ({} positions)", positions.size());
            for pos in positions {
                eprintln!("\t\t- {pos:?}");
            }
        }
        let adj = ctx.potential_cooperation.at(t);
        eprintln!("\tEdges:");
        for e in adj.edges() {
            eprintln!("\t\t- {e:?}");
        }
        for &(helper, beneficiary) in edges {
            assert!(
                adj.contains(helper, beneficiary),
                "({helper}, {beneficiary}) should be present at t={t}"
            );
        }
        assert_eq!(edges.len(), adj.n_edges());
    }
    let t_max = 9;
    let mut ctx = ConstraintContext::new(&world, t_max);
    let mut edges = vec![];
    // t=0 -> nothing
    assert!(ctx.potential_cooperation.at(0).is_empty());
    // t=1 -> agent 0 can help 1 and 2
    edges.extend([(0, 1), (0, 2)]);
    verify_edges(1, &mut ctx, &edges);

    // At t=2, 1 can help 2
    // Importantly, agent 1 can **NOT** help agent 0 because agent 0 cannot reach any laser tile of
    // colour 1 at t=2 (tile 0 of a beam is forbidden for foreign coloured agents, and foreign start
    //  tiles are forbidden at t=1).
    edges.push((1, 2));
    verify_edges(2, &mut ctx, &edges);

    // At t=3, 1 can also help 0
    edges.push((1, 0));
    verify_edges(3, &mut ctx, &edges);
    // At t=4, 2 can help 1
    edges.push((2, 1));
    verify_edges(4, &mut ctx, &edges);
    // At t=5, 2 can help 0
    edges.push((2, 0));
    verify_edges(5, &mut ctx, &edges);
    // At t=6, remains identical
    verify_edges(6, &mut ctx, &edges);
    // At t=7, Agent 0 can no longer block its laser, or it will be late to the exit
    edges.retain(|&(h, _)| h != 0); // Only retain edges where the helper is not 0
    verify_edges(7, &mut ctx, &edges);
    // At t=8, Agent 1 can no longer block its laser, or it will be late to the exit
    edges.retain(|&(h, _)| h != 1); // Only retain edges where the helper is not 1
    verify_edges(8, &mut ctx, &edges);
    // At t=t_max, all agents must be on their respective exits
    edges.clear();
    assert!(ctx.potential_cooperation.at(t_max).is_empty());
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

/// Check that all possible trails are enumerated on a 3-agents world
/// This test is there for a potential future optimization where only realistic
/// trails are followed. In this example, combinations [0,2,0], [2,0,0], [2,0,1]
/// and [0,2,1] are actually impossible to achieve and should therefore be absent
/// of the enumerated trails.
///
/// Such optimization is not yet implemented (and may never be).
#[test]
#[ignore = "This optimization not implemented yet"]
fn potential_graph_3agents_trails_of_length2_optimized() {
    let world = World::try_from(
        "
 @  S0  S1  S2
L0E  .   .  .
L1E  .   .  .
L2E  .   .  .
 @   X   X  X
",
    )
    .unwrap();
    let t_max = 10;
    let mut ctx = ConstraintContext::new(&world, t_max);
    ctx.update(t_max);
    let graph = ctx.potential_cooperation;

    fn to_vertex_trail(r: Vec<Vec<PotentialHelpEdge>>) -> Vec<Vec<usize>> {
        r.into_iter()
            .map(|trail| {
                std::iter::once(trail[0].helper)
                    .chain(trail.iter().map(|e| e.beneficiary))
                    .collect()
            })
            .collect()
    }
    let trails = graph.enumerate_trails_of_length(2);
    let trails = to_vertex_trail(trails);
    let unrealistic_trails = [vec![0, 2, 0], vec![2, 0, 0], vec![2, 0, 1], vec![0, 2, 1]];
    for trail in unrealistic_trails {
        assert!(!trails.contains(&trail));
    }
}

#[test]
fn potential_graph_3agents_trails_of_length2() {
    let world = World::try_from(
        "
 @  S0  S1  S2
L0E  .   .  .
L1E  .   .  .
L2E  .   .  .
 @   X   X  X
",
    )
    .unwrap();
    let t_max = 10;
    let mut ctx = ConstraintContext::new(&world, t_max);
    ctx.update(t_max);
    let graph = ctx.potential_cooperation;

    fn to_vertex_trail(r: Vec<Vec<PotentialHelpEdge>>) -> Vec<Vec<usize>> {
        r.into_iter()
            .map(|trail| {
                std::iter::once(trail[0].helper)
                    .chain(trail.iter().map(|e| e.beneficiary))
                    .collect()
            })
            .collect()
    }

    let mut valid = vec![];
    for a1 in 0..world.n_agents() {
        for a2 in 0..world.n_agents() {
            for a3 in 0..world.n_agents() {
                if a1 != a2 && a2 != a3 {
                    valid.push(vec![a1, a2, a3])
                }
            }
        }
    }
    let trails = graph.enumerate_trails_of_length(2);
    let trails = to_vertex_trail(trails);
    for trail in valid {
        assert!(trails.contains(&trail));
    }
}

#[test]
fn cooperation_at_t0() {
    let world = World::try_from(
        "
    L0E S0 S1
     @  X  X",
    )
    .unwrap();
    let mut ctx = ConstraintContext::new(&world, 5);
    ctx.update(0);
    let graph = ctx.potential_cooperation;
    assert!(!graph.at(0).is_empty());
}

/// Per-step trail enumeration partitions full enumeration by the final edge time.
///
/// @ai-generated
#[test]
fn trails_ending_at_matches_full_enumeration() {
    let graphs = [
        graph_from(3, &[&[(0, 1)], &[(1, 2)]]),
        graph_from(3, &[&[(0, 1), (0, 2)], &[(1, 2), (2, 1)]]),
        graph_from(3, &[&[(0, 1), (1, 0), (1, 2)]]),
    ];

    for graph in graphs {
        for length in 1..=3 {
            let full = graph
                .enumerate_trails_of_length(length)
                .into_iter()
                .collect::<HashSet<_>>();
            let partitioned = (0..=graph.horizon())
                .flat_map(|t| graph.enumerate_trails_ending_at(length, t))
                .collect::<HashSet<_>>();
            assert_eq!(partitioned, full);
        }
    }
}

/// Ending-at enumeration supports simultaneous edges, repeated agents, and rejects length zero.
///
/// @ai-generated
#[test]
fn trails_ending_at_handles_edge_cases() {
    let simultaneous = graph_from(2, &[&[(0, 1), (1, 0)]]);
    assert!(simultaneous.enumerate_trails_ending_at(0, 0).is_empty());
    assert_eq!(simultaneous.enumerate_trails_ending_at(2, 0).len(), 2);

    let across_time = graph_from(3, &[&[(0, 1)], &[(1, 2)]]);
    assert!(across_time.enumerate_trails_ending_at(2, 0).is_empty());
    assert_eq!(across_time.enumerate_trails_ending_at(2, 1).len(), 1);

    let revisiting = graph_from(2, &[&[(0, 1)], &[(1, 0)], &[(0, 1)]]);
    let trails = revisiting.enumerate_trails_ending_at(3, 2);
    assert_eq!(trails.len(), 1);
    assert_eq!(
        trails[0].iter().map(|edge| edge.t).collect::<Vec<_>>(),
        vec![0, 1, 2]
    );
}
