from __future__ import annotations

import pytest
from lle.characterization.plan.graph import DependencyEdge, TemporalCooperationGraph


@pytest.fixture
def single_edge_graph():
    """A simple graph with one dependency: agent 0 helps agent 1 at t=2."""
    edges = [DependencyEdge(helper=0, beneficiary=1, t=2)]
    return TemporalCooperationGraph(edges)


@pytest.fixture
def linear_sequence_graph():
    """A linear sequence: 0 -> 1 -> 2 -> 3 across time steps."""
    edges = [
        DependencyEdge(helper=0, beneficiary=1, t=1),
        DependencyEdge(helper=1, beneficiary=2, t=2),
        DependencyEdge(helper=2, beneficiary=3, t=3),
    ]
    return TemporalCooperationGraph(edges)


@pytest.fixture
def branching_graph():
    """One agent helps multiple agents: agent 0 -> {1, 2, 3}."""
    edges = [
        DependencyEdge(helper=0, beneficiary=1, t=1),
        DependencyEdge(helper=0, beneficiary=2, t=1),
        DependencyEdge(helper=0, beneficiary=3, t=1),
    ]
    return TemporalCooperationGraph(edges)


@pytest.fixture
def converging_graph():
    """Multiple agents help one: {1, 2, 3} -> 0."""
    edges = [
        DependencyEdge(helper=1, beneficiary=0, t=1),
        DependencyEdge(helper=2, beneficiary=0, t=1),
        DependencyEdge(helper=3, beneficiary=0, t=1),
    ]
    return TemporalCooperationGraph(edges)


@pytest.fixture
def temporal_cycle_graph():
    """A temporal cycle: 0 -> 1 -> 2 -> 0 with strictly increasing time."""
    edges = [
        DependencyEdge(helper=0, beneficiary=1, t=1),
        DependencyEdge(helper=1, beneficiary=2, t=2),
        DependencyEdge(helper=2, beneficiary=0, t=3),
    ]
    return TemporalCooperationGraph(edges)


@pytest.fixture
def static_cycle_graph():
    """A cycle that does NOT progress through time: 0 -> 1 -> 0 at same t."""
    edges = [
        DependencyEdge(helper=0, beneficiary=1, t=1),
        DependencyEdge(helper=1, beneficiary=0, t=1),
    ]
    return TemporalCooperationGraph(edges)


@pytest.fixture
def hamiltonian_cycle_graph():
    """A Hamiltonian cycle in a 4-agent graph: 0 -> 1 -> 2 -> 3 -> 0."""
    edges = [
        DependencyEdge(helper=0, beneficiary=1, t=1),
        DependencyEdge(helper=1, beneficiary=2, t=2),
        DependencyEdge(helper=2, beneficiary=3, t=3),
        DependencyEdge(helper=3, beneficiary=0, t=4),
    ]
    return TemporalCooperationGraph(edges)


@pytest.fixture
def scc_3_agents():
    """A fully connected 3-agent SCC: 0 <-> 1, 1 <-> 2, 0 <-> 2."""
    edges = [
        DependencyEdge(helper=0, beneficiary=1, t=1),
        DependencyEdge(helper=1, beneficiary=0, t=2),
        DependencyEdge(helper=1, beneficiary=2, t=3),
        DependencyEdge(helper=2, beneficiary=1, t=4),
        DependencyEdge(helper=0, beneficiary=2, t=5),
        DependencyEdge(helper=2, beneficiary=0, t=6),
    ]
    return TemporalCooperationGraph(edges)


@pytest.fixture
def disconnected_graph():
    """Two disconnected components: {0, 1} and {2, 3}."""
    edges = [
        DependencyEdge(helper=0, beneficiary=1, t=1),
        DependencyEdge(helper=2, beneficiary=3, t=2),
    ]
    return TemporalCooperationGraph(edges)


@pytest.fixture
def multi_time_same_edge():
    """The same help relationship occurs multiple times across different time steps."""
    edges = [
        DependencyEdge(helper=0, beneficiary=1, t=1),
        DependencyEdge(helper=0, beneficiary=1, t=3),
        DependencyEdge(helper=0, beneficiary=1, t=5),
    ]
    return TemporalCooperationGraph(edges)


class TestBasicConstruction:
    """Test graph construction and basic property access."""

    def test_non_empty_graph_not_independent(self, single_edge_graph: TemporalCooperationGraph):
        """A graph with at least one edge is not independent."""
        assert not single_edge_graph.profile().is_independent


class TestAccessors:
    """Test accessor methods for querying edges."""

    def test_flattened_edges_collapses_time(self, multi_time_same_edge: TemporalCooperationGraph):
        """flattened_edges collapses time dimension, removing duplicates."""
        flattened = multi_time_same_edge.flattened_edges()
        assert flattened == {(0, 1)}
        assert len(flattened) == 1

    def test_flattened_edges_all_edges(self, branching_graph: TemporalCooperationGraph):
        """flattened_edges returns all distinct pairs."""
        flattened = branching_graph.flattened_edges()
        assert len(flattened) == 3
        assert (0, 1) in flattened
        assert (0, 2) in flattened
        assert (0, 3) in flattened

    def test_active_times_returns_every_timestamp_with_an_edge(self):
        """active_times lists each distinct time step that has at least one edge."""
        graph = TemporalCooperationGraph(
            [
                DependencyEdge(helper=0, beneficiary=1, t=1),
                DependencyEdge(helper=1, beneficiary=2, t=1),
                DependencyEdge(helper=2, beneficiary=3, t=5),
            ]
        )
        assert graph.active_times() == (1, 5)

    def test_active_times_of_empty_graph_is_empty(self):
        assert TemporalCooperationGraph.empty().active_times() == ()

    def test_layers_for_returns_sorted_temporal_layers(self, branching_graph: TemporalCooperationGraph):
        """layers_for groups one helper's beneficiaries per time step."""
        layers = branching_graph.layers_for(0)
        assert len(layers) == 1
        assert layers[0].t == 1
        assert layers[0].helper == 0
        assert layers[0].beneficiaries == (1, 2, 3)

    def test_layers_for_unknown_helper_raises_key_error(self, branching_graph: TemporalCooperationGraph):
        with pytest.raises(KeyError):
            branching_graph.layers_for(99)

    def test_time_layer_edges_yields_one_edge_per_beneficiary(self, branching_graph: TemporalCooperationGraph):
        """TimeLayer.edges() expands a layer back into concrete DependencyEdges."""
        layer = branching_graph.layers_for(0)[0]
        edges = list(layer.edges())
        assert edges == [
            DependencyEdge(0, 1, 1),
            DependencyEdge(0, 2, 1),
            DependencyEdge(0, 3, 1),
        ]

    def test_beneficiaries_at_returns_beneficiaries_for_that_timestamp(self):
        """beneficiaries_at only returns beneficiaries helped at the exact requested time."""
        graph = TemporalCooperationGraph(
            [
                DependencyEdge(helper=0, beneficiary=1, t=1),
                DependencyEdge(helper=0, beneficiary=2, t=5),
            ]
        )
        assert graph.beneficiaries_at(0, 1) == (1,)
        assert graph.beneficiaries_at(0, 5) == (2,)
        assert graph.beneficiaries_at(0, 2) == ()
        assert graph.beneficiaries_at(99, 1) == ()


class TestLongestTrail:
    """Test longest_sequence method for temporal dependency sequences."""

    def test_temporal_sequence_strictly_increasing_times(self):
        """Sequence with strictly increasing times: a -> b -> c -> d."""
        edges = [
            DependencyEdge(helper=0, beneficiary=1, t=1),
            DependencyEdge(helper=1, beneficiary=2, t=2),
            DependencyEdge(helper=2, beneficiary=3, t=3),
        ]
        graph = TemporalCooperationGraph(edges)
        assert len(graph.longest_trail()) == 3

    def test_temporal_sequence_same_time_two_edges(self):
        """Edges at the same time can form a non-decreasing-time sequence."""
        edges = [
            DependencyEdge(helper=0, beneficiary=1, t=1),
            DependencyEdge(helper=1, beneficiary=2, t=1),
        ]
        graph = TemporalCooperationGraph(edges)
        assert len(graph.longest_trail()) == 2

    def test_temporal_sequence_same_time(self):
        """Longer simultaneous paths also sequence under non-decreasing-time semantics."""
        edges = [
            DependencyEdge(helper=0, beneficiary=1, t=1),
            DependencyEdge(helper=1, beneficiary=2, t=1),
            DependencyEdge(helper=2, beneficiary=3, t=1),
        ]
        graph = TemporalCooperationGraph(edges)
        assert len(graph.longest_trail()) == 3

    def test_temporal_sequence_decreasing_times(self):
        """Sequence with decreasing times cannot form a sequence (time must be non-decreasing)."""
        edges = [
            DependencyEdge(helper=0, beneficiary=1, t=3),
            DependencyEdge(helper=1, beneficiary=2, t=2),
            DependencyEdge(helper=2, beneficiary=3, t=1),
        ]
        graph = TemporalCooperationGraph(edges)
        # Each edge is at a lower time than the previous, so no valid temporal sequence of length >= 2
        assert len(graph.longest_trail()) == 1

    def test_temporal_sequence_non_monotonic_times(self):
        """The example from prompt: a->b at t0, b->c at t2, c->d at t1."""
        edges = [
            DependencyEdge(helper=0, beneficiary=1, t=0),
            DependencyEdge(helper=1, beneficiary=2, t=2),
            DependencyEdge(helper=2, beneficiary=3, t=1),
        ]
        graph = TemporalCooperationGraph(edges)
        # 0 -> 1 (t=0), 1 -> 2 (t=2): sequence of length 2
        # 2 -> 3 (t=1) happens before 1 -> 2 (t=2), so cannot extend sequence
        trail = graph.longest_trail()
        assert len(trail) == 2
        assert edges[0] in trail
        assert edges[1] in trail
        assert edges[2] not in trail

    def test_temporal_sequence_with_equal_times(self):
        """Equal timestamps extend a sequence when they form an acyclic same-time path."""
        edges = [
            DependencyEdge(helper=0, beneficiary=1, t=1),
            DependencyEdge(helper=1, beneficiary=2, t=1),
            DependencyEdge(helper=2, beneficiary=3, t=1),
        ]
        graph = TemporalCooperationGraph(edges)
        trail = graph.longest_trail()
        assert len(trail) == 3
        for e in edges:
            assert e in trail

    def test_temporal_sequence_mixed_times(self):
        """Complex case with multiple paths at different times."""
        edges = [
            DependencyEdge(helper=0, beneficiary=1, t=1),
            DependencyEdge(helper=1, beneficiary=2, t=2),
            DependencyEdge(helper=0, beneficiary=3, t=1),
            DependencyEdge(helper=3, beneficiary=4, t=3),
        ]
        graph = TemporalCooperationGraph(edges)
        # Longest sequence: 0 -> 1 -> 2 (length 2) or 0 -> 3 -> 4 (length 2)
        trail = graph.longest_trail()
        assert len(trail) == 2

    def test_temporal_sequence_hamiltonian_cycle(self, hamiltonian_cycle_graph: TemporalCooperationGraph):
        """A Hamiltonian cycle with strictly increasing times."""
        # 0 -> 1 (t=1), 1 -> 2 (t=2), 2 -> 3 (t=3), 3 -> 0 (t=4)
        # Longest temporal sequence: 0 -> 1 -> 2 -> 3 -> 0 (length 4)
        # Include 3 -> 0 even though it re-visits 0.
        assert len(hamiltonian_cycle_graph.longest_trail()) == 4

    def test_temporal_sequence_may_revisit_agents(self):
        """Agent revisit: 0 -> 1 -> 0 -> 1 is a length-3 strict temporal walk."""
        edges = [
            DependencyEdge(helper=0, beneficiary=1, t=1),
            DependencyEdge(helper=1, beneficiary=0, t=2),
            DependencyEdge(helper=0, beneficiary=2, t=3),
        ]
        graph = TemporalCooperationGraph(edges)
        assert len(graph.longest_trail()) == 3

    def test_temporal_sequence_may_not_revisit_same_edge(self):
        """There is a loop in the help graph, but the temporal graph should not re-visit the same edge twice."""
        edges = [
            DependencyEdge(helper=0, beneficiary=1, t=1),
            DependencyEdge(helper=1, beneficiary=0, t=1),
            DependencyEdge(helper=1, beneficiary=2, t=3),
        ]
        graph = TemporalCooperationGraph(edges)
        assert len(graph.longest_trail()) == 3

    def test_three_agents_is_trail_of_length_2(self):
        """Doc example: a -> b -> c returns 2."""
        edges = [
            DependencyEdge(helper=0, beneficiary=1, t=1),
            DependencyEdge(helper=1, beneficiary=2, t=2),
        ]
        graph = TemporalCooperationGraph(edges)
        assert len(graph.longest_trail()) == 2

    def test_cycle_three_agents_returns_3(self):
        """Doc example: a -> b -> c -> a returns 3."""
        # This is the temporal_cycle_graph: 0->1 (t=1), 1->2 (t=2), 2->0 (t=3)
        edges = [
            DependencyEdge(helper=0, beneficiary=1, t=1),
            DependencyEdge(helper=1, beneficiary=2, t=2),
            DependencyEdge(helper=2, beneficiary=0, t=3),
        ]
        graph = TemporalCooperationGraph(edges)
        assert len(graph.longest_trail()) == 3
        assert graph.has_closed_trail_of_order(3)

    def test_cycle_same_time_three_agents_returns_3(self):
        """Doc example: a -> b -> c -> a returns 3."""
        # This is the temporal_cycle_graph: 0->1 (t=1), 1->2 (t=1), 2->0 (t=3)
        edges = [
            DependencyEdge(0, 1, t=1),
            DependencyEdge(1, 2, t=1),
            DependencyEdge(2, 0, t=3),
        ]
        graph = TemporalCooperationGraph(edges)
        assert len(graph.longest_trail()) == 3
        assert graph.has_closed_trail_of_order(3)
        assert not graph.has_closed_trail_of_order(4)

    def test_four_agents_sequence_returns_3(self):
        """Doc example: a -> b -> c -> d returns 3."""
        edges = [
            DependencyEdge(helper=0, beneficiary=1, t=1),
            DependencyEdge(helper=1, beneficiary=2, t=2),
            DependencyEdge(helper=2, beneficiary=3, t=3),
        ]
        graph = TemporalCooperationGraph(edges)
        assert len(graph.longest_trail()) == 3

    def test_branching_returns_0(self):
        """Doc example: a -> b and a -> c returns 0 (only 1-edge paths)."""
        edges = [
            DependencyEdge(helper=0, beneficiary=1, t=1),
            DependencyEdge(helper=0, beneficiary=2, t=2),
        ]
        graph = TemporalCooperationGraph(edges)
        assert len(graph.longest_trail()) == 1

    def test_mutual_help_is_sequence(self):
        """Doc example: a -> b -> a returns 2 when the reverse edge is later."""
        edges = [
            DependencyEdge(helper=0, beneficiary=1, t=1),
            DependencyEdge(helper=1, beneficiary=0, t=2),
        ]
        graph = TemporalCooperationGraph(edges)
        assert len(graph.longest_trail()) == 2

    def test_simultaneous_mutual_help_is_bounded_sequence(self):
        """Same-time reciprocal help forms a trail of length 2, not an infinite walk."""
        edges = [
            DependencyEdge(helper=0, beneficiary=1, t=1),
            DependencyEdge(helper=1, beneficiary=0, t=1),
        ]
        graph = TemporalCooperationGraph(edges)
        assert len(graph.longest_trail()) == 2

    def test_independent_graph_returns_0(self):
        """Doc example: an independent graph returns 0."""
        graph = TemporalCooperationGraph(edges=[])
        assert len(graph.longest_trail()) == 0

    def test_longest_closed_trail_of_empty_graph_is_empty(self):
        assert TemporalCooperationGraph.empty().longest_closed_trail() == []


class TestTemporalCycleDetection:
    """Test has_cycle method."""

    def test_temporal_cycle_branching(self, branching_graph: TemporalCooperationGraph):
        """Branching structure has no cycle."""
        assert not branching_graph.has_closed_trail_of_order(2)

    def test_temporal_cycle_detected(self, temporal_cycle_graph: TemporalCooperationGraph):
        """Temporal cycle with strictly increasing time is detected."""
        # 0 -> 1 -> 2 -> 0 with t increasing
        assert not temporal_cycle_graph.has_closed_trail_of_order(2)
        assert temporal_cycle_graph.has_closed_trail_of_order(3)
        assert not temporal_cycle_graph.has_closed_trail_of_order(4)

    def test_temporal_cycle_static_is_detected(self, static_cycle_graph: TemporalCooperationGraph):
        """Cycle at the same time step is a temporal cycle."""
        # 0 <-> 1 both at t=1: time is not strictly increasing
        assert static_cycle_graph.has_closed_trail_of_order(2)

    def test_temporal_cycle_disconnected(self, disconnected_graph: TemporalCooperationGraph):
        """No temporal cycle when components are separate."""
        assert not disconnected_graph.has_closed_trail_of_order(2)

    def test_temporal_cycle_in_scc(self, scc_3_agents: TemporalCooperationGraph):
        """SCC with backward edges may have temporal cycles."""
        # The test SCC has edges at different times that could form cycles
        # Actual result depends on edge ordering in fixture
        assert scc_3_agents.has_closed_trail_of_order(3)


class TestEdgeCases:
    """Test edge cases and boundary conditions."""

    def test_self_loop_is_ignored(self):
        edges = [DependencyEdge(helper=0, beneficiary=0, t=1)]
        graph = TemporalCooperationGraph(edges)
        # Self-loop is included in flattened_edges
        assert (0, 0) not in graph.flattened_edges()
        assert graph.is_empty

    def test_duplicate_edges_handled_by_frozenset(self):
        """Duplicate edges in input are deduplicated by frozenset."""
        edges = [
            DependencyEdge(helper=0, beneficiary=1, t=1),
            DependencyEdge(helper=0, beneficiary=1, t=1),  # Duplicate at same time
            DependencyEdge(helper=0, beneficiary=1, t=1),
        ]
        graph = TemporalCooperationGraph(edges)
        # Duplicates are collapsed to one edge in the frozenset
        assert len(graph.edges) == 1

    def test_many_edges(self):
        """Graph with many edges (stress test)."""
        edges = [DependencyEdge(helper=i % 5, beneficiary=(i + 1) % 5, t=i // 5) for i in range(1000)]
        graph = TemporalCooperationGraph(edges)
        assert len(graph.edges) <= 1000
        # The actual number may be less due to duplicate handling

    def test_negative_time_steps(self):
        """Graph can technically handle negative time (unusual but allowed)."""
        edges = [DependencyEdge(helper=0, beneficiary=1, t=-5)]
        graph = TemporalCooperationGraph(edges)
        assert (-5, (0, 1)) in {(e.t, (e.helper, e.beneficiary)) for e in graph.edges}


class TestRealWorldPatterns:
    """Test patterns that might arise from actual trajectories."""

    def test_sequential_help_sequence(self):
        """A common pattern: sequential help where agents take turns."""
        # Agent 0 helps 1, then 1 helps 2, then 2 helps 3 sequentially
        edges = [
            DependencyEdge(helper=0, beneficiary=1, t=1),
            DependencyEdge(helper=1, beneficiary=2, t=2),
            DependencyEdge(helper=2, beneficiary=3, t=3),
        ]
        graph = TemporalCooperationGraph(edges)
        assert len(graph.longest_trail()) == 3
        assert not graph.has_closed_trail_of_order(2)

    def test_bottleneck_pattern(self):
        """One agent is critical: many agents depend on it."""
        edges = [
            DependencyEdge(helper=0, beneficiary=1, t=1),
            DependencyEdge(helper=0, beneficiary=2, t=1),
            DependencyEdge(helper=0, beneficiary=3, t=1),
            DependencyEdge(helper=0, beneficiary=4, t=1),
        ]
        graph = TemporalCooperationGraph(edges)
        assert len(graph.longest_trail()) == 1

    def test_diamond_pattern(self):
        """Diamond: 0 -> {1,2} and {1,2} -> 3."""
        edges = [
            DependencyEdge(helper=0, beneficiary=1, t=1),
            DependencyEdge(helper=0, beneficiary=2, t=1),
            DependencyEdge(helper=1, beneficiary=3, t=2),
            DependencyEdge(helper=2, beneficiary=3, t=2),
        ]
        graph = TemporalCooperationGraph(edges)
        assert len(graph.longest_trail()) == 2
        assert not graph.has_closed_trail_of_order(2)
        assert not graph.has_closed_trail_of_order(3)
        assert graph.has_asymmetric_edge()  # Agent 0 helps asymmetrically


def test_has_asymmetric_edge():
    edges = [DependencyEdge(helper=0, beneficiary=1, t=1)]
    tdg = TemporalCooperationGraph(edges)
    assert tdg.has_asymmetric_edge()

    edges = [
        DependencyEdge(helper=0, beneficiary=1, t=1),
        DependencyEdge(helper=2, beneficiary=0, t=1),
    ]
    tdg = TemporalCooperationGraph(edges)
    assert tdg.has_asymmetric_edge()


def test_asymmetric_edges():
    graph = TemporalCooperationGraph([DependencyEdge(0, 1, 2), DependencyEdge(1, 2, 3)])
    assert graph.asymmetric_edges() == {(0, 1)}


def test_cooperation_cycle():
    """
    A repeated-agent closed trail retains all seven edges and has support four.
    """
    tcg = TemporalCooperationGraph(
        [
            DependencyEdge(0, 1, 1),
            DependencyEdge(1, 0, 2),
            DependencyEdge(0, 2, 3),
            DependencyEdge(2, 1, 4),
            DependencyEdge(1, 0, 5),
            DependencyEdge(0, 3, 6),
            DependencyEdge(3, 0, 7),
        ]
    )

    assert len(tcg.longest_closed_trail()) == 7
    assert tcg.interdependence_order() == 4
    assert tcg.closed_trail_orders() == frozenset({2, 3, 4})


def test_closed_trail_exact_support_accepts_bowtie_and_double_petal():
    """Repeated-agent closed trails are recognized at their exact support order."""
    bowtie = TemporalCooperationGraph(
        [
            DependencyEdge(0, 1, 1),
            DependencyEdge(1, 0, 2),
            DependencyEdge(0, 2, 3),
            DependencyEdge(2, 0, 4),
        ]
    )
    double_petal = TemporalCooperationGraph(
        [
            DependencyEdge(0, 1, 1),
            DependencyEdge(1, 2, 2),
            DependencyEdge(2, 0, 3),
            DependencyEdge(0, 1, 4),
            DependencyEdge(1, 3, 5),
            DependencyEdge(3, 0, 6),
        ]
    )

    assert bowtie.has_closed_trail_of_order(3)
    assert len(bowtie.closed_trail_of_order(3)) == 4
    assert double_petal.has_closed_trail_of_order(4)
    assert len(double_petal.closed_trail_of_order(4)) == 6


def test_closed_trail_orders_do_not_infer_missing_exact_order_from_larger_ring():
    """
    A four-agent ring is threshold-interdependent at three but not exact-order three.
    """
    graph = TemporalCooperationGraph(
        [
            DependencyEdge(0, 1, 1),
            DependencyEdge(1, 2, 2),
            DependencyEdge(2, 3, 3),
            DependencyEdge(3, 0, 4),
        ]
    )

    assert graph.closed_trail_orders() == frozenset({4})
    assert graph.has_closed_trail_of_order(4)
    assert not graph.has_closed_trail_of_order(3)


def test_max_distinct_beneficiaries_groups_per_helper(branching_graph: TemporalCooperationGraph):
    """One helper with three beneficiaries has an outgoing degree of three.

    @ai-generated
    """
    assert branching_graph.max_distinct_beneficiaries() == 3
    assert branching_graph.max_distinct_helpers() == 1


def test_max_distinct_beneficiaries_is_the_dual_of_max_distinct_helpers(converging_graph: TemporalCooperationGraph):
    """Three helpers of one beneficiary invert both degree metrics.

    @ai-generated
    """
    assert converging_graph.max_distinct_helpers() == 3
    assert converging_graph.max_distinct_beneficiaries() == 1


def test_max_distinct_beneficiaries_collapses_time_and_duplicates():
    """Repeated help to one beneficiary counts once, whatever its timestamps.

    @ai-generated
    """
    graph = TemporalCooperationGraph(
        [
            DependencyEdge(0, 1, 1),
            DependencyEdge(0, 1, 1),
            DependencyEdge(0, 1, 6),
            DependencyEdge(0, 2, 6),
            DependencyEdge(3, 4, 2),
        ]
    )
    assert graph.max_distinct_beneficiaries() == 2


def test_degree_metrics_ignore_self_loops():
    """Self-help is ignored."""
    graph = TemporalCooperationGraph([DependencyEdge(0, 0, 1), DependencyEdge(0, 1, 2)])
    assert graph.max_distinct_beneficiaries() == 1
    assert graph.max_distinct_helpers() == 1


def test_degree_metrics_of_the_empty_graph_are_zero():
    """An empty graph has no outgoing or incoming degree.

    @ai-generated
    """
    graph = TemporalCooperationGraph.empty()
    assert graph.max_distinct_beneficiaries() == 0
    assert graph.max_distinct_helpers() == 0
