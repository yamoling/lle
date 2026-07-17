import lle
from lle import World
from lle.characterization import profile_plan
from lle.characterization.plan import DependencyEdge, TemporalCooperationGraph


def test_profile_empty_graph():
    """Profile of empty graph has correct independent flag."""
    empty_graph = TemporalCooperationGraph.empty()
    profile = empty_graph.profile()
    assert profile.is_independent
    assert not profile.is_cooperative
    assert not profile.is_interdependent()
    assert not profile.is_chained()
    assert not profile.is_asymmetric


def test_profile_single_edge():
    """Profile of single-edge graph has correct metrics."""
    single_edge_graph = TemporalCooperationGraph([DependencyEdge(0, 1, 16)])
    profile = single_edge_graph.profile()
    assert not profile.is_independent
    assert profile.is_cooperative
    assert not profile.is_interdependent()
    assert not profile.is_chained()
    assert profile.is_asymmetric


def test_profile_joining_edges():
    graph = TemporalCooperationGraph(
        [
            DependencyEdge(5, 6, 5),
            DependencyEdge(6, 9, 18),
            DependencyEdge(9, 52, 125),
            DependencyEdge(4, 52, 17),  # Here for noise
        ]
    )
    profile = graph.profile()
    assert profile.is_chained()
    assert profile.is_chained(3)
    assert not profile.is_chained(4)


def test_profile_separating_edges():
    """
         -- 1->3, 3->6
        /
    0->1
        \
         -- 1->5, 5->2, 2->9
    """
    graph = TemporalCooperationGraph(
        [
            DependencyEdge(0, 1, 1),
            # Top path
            DependencyEdge(1, 3, 2),
            DependencyEdge(3, 6, 3),
            # Bottom path
            DependencyEdge(1, 5, 25),
            DependencyEdge(5, 2, 26),
            DependencyEdge(2, 9, 50),
        ]
    )
    profile = graph.profile()
    assert profile.is_cooperative
    assert profile.is_asymmetric
    for i in [2, 3, 4]:
        assert profile.is_chained(i)
    assert not profile.is_chained(5)
    assert not profile.is_interdependent(2)
    assert not profile.is_mutual


def test_profile_two_mutual():
    """Profile has_mutual_help property is True for SCCs."""
    edges = [
        DependencyEdge(helper=0, beneficiary=1, t=1),
        DependencyEdge(helper=1, beneficiary=0, t=1),
    ]
    static_cycle_graph = TemporalCooperationGraph(edges)
    profile = static_cycle_graph.profile()
    assert profile.is_cooperative
    assert not profile.is_independent
    assert profile.is_interdependent(2)
    assert not profile.is_interdependent(3)
    assert profile.is_chained(2)
    assert not profile.is_chained(3)
    assert profile.is_mutual


def test_profile_length5_chain():
    # 0->1, 1->15, 15->19, 19->17, 17->8
    linear_chain_graph = TemporalCooperationGraph(
        [  # Shuffled
            DependencyEdge(19, 17, 125),
            DependencyEdge(0, 1, 0),
            DependencyEdge(17, 8, 126),
            DependencyEdge(15, 19, 19),
            DependencyEdge(1, 15, 14),
        ]
    )
    profile = linear_chain_graph.profile()
    assert not profile.is_mutual
    assert profile.is_cooperative
    assert not profile.is_independent
    for i in range(2, 6):
        assert profile.is_chained(i)
        assert not profile.is_interdependent(i)
    assert not profile.is_chained(6)


def test_simultaneous_mutual_help_is_bounded_chain():
    """Same-time reciprocal help forms a trail of length 2, not an infinite walk."""
    edges = [
        DependencyEdge(helper=0, beneficiary=1, t=1),
        DependencyEdge(helper=1, beneficiary=0, t=1),
    ]
    graph = TemporalCooperationGraph(edges)
    profile = graph.profile()
    assert profile.is_chained()
    assert profile.is_cooperative
    assert not profile.is_independent
    assert profile.is_interdependent()


def test_trajectory_profile_rejects_mutual_as_asymmetric():
    graph = TemporalCooperationGraph(
        edges=[
            DependencyEdge(0, 1, 1),
            DependencyEdge(1, 0, 2),
        ]
    )
    profile = graph.profile()
    assert not profile.is_asymmetric
    assert profile.is_mutual


def test_not_interdependent():
    """
    In the below world, the agents are not 3-interdependent because laser L1W
    can be avoided.
    """
    world = World("""
    S0  .  .  .  .  @ @
    S1  .  .  .  .  @ @
    S2 L0E .  .  .  @ @
    .  .  .  @  .  . .
    .  .  .  . L1W . .
    .  @ L2E .  .  . .
    .  @  @  .  X  X X
    """)
    plan = lle.solve(world, 16)
    assert plan is not None
    profile = profile_plan(world, plan)
    assert profile.is_cooperative
    assert profile.is_chained
    assert not profile.is_independent
    assert profile.is_interdependent(2)
    assert not profile.is_interdependent(3)


def test_multiple_stepd_in_a_row_remains_asymmetric():
    """Agent 0 must block the same laser beam during multiple time steps
    for agent 1, but this should still be considered to be asymmetric.
    """
    world = World("""
 @  S0 S1 @ @ @
L0E .  .  . . .
 @  @  @  @ @ .
 @  @  @  @ X .
 @  @  @  @ @ X
""")
    plan = lle.solve(world, 16)
    assert plan is not None
    profile = profile_plan(world, plan)
    assert profile.is_asymmetric
    assert profile.is_cooperative
    assert not profile.is_independent
    assert not profile.is_chained(2)
    assert not profile.is_interdependent(2)
    assert not profile.is_interdependent(3)
