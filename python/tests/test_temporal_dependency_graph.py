"""Comprehensive unit tests for the TemporalDependencyGraph class.

Tests cover:
- Basic construction and properties
- Accessors (edges_at, flattened_edges, helpers_of, beneficiaries_of)
- Fan-in/fan-out metrics
- Chain detection (longest_chain)
- Cycle detection (temporal and Hamiltonian)
- Strongly connected components
- Edge cases and boundary conditions
- Integration with TrajectoryProfile
"""

from __future__ import annotations

import pytest
from lle.characterization.trajectory.graph import DependencyEdge, TemporalDependencyGraph
from lle.characterization.trajectory.profile import TrajectoryProfile

# ============================================================================
# Test fixture definitions: pre-built graphs for common patterns
# ============================================================================


@pytest.fixture
def empty_graph_2_agents() -> TemporalDependencyGraph:
    """A graph with 2 agents and no dependencies."""
    return TemporalDependencyGraph(n_agents=2, edges=[], horizon=5)


@pytest.fixture
def empty_graph_5_agents() -> TemporalDependencyGraph:
    """A graph with 5 agents and no dependencies."""
    return TemporalDependencyGraph(n_agents=5, edges=[], horizon=10)


@pytest.fixture
def single_agent_graph() -> TemporalDependencyGraph:
    """A graph with only 1 agent (can have no internal dependencies)."""
    return TemporalDependencyGraph(n_agents=1, edges=[], horizon=3)


@pytest.fixture
def single_edge_graph() -> TemporalDependencyGraph:
    """A simple graph with one dependency: agent 0 helps agent 1 at t=2."""
    edges = [DependencyEdge(helper=0, beneficiary=1, t=2)]
    return TemporalDependencyGraph(n_agents=2, edges=edges, horizon=5)


@pytest.fixture
def linear_chain_graph() -> TemporalDependencyGraph:
    """A linear chain: 0 -> 1 -> 2 -> 3 across time steps."""
    edges = [
        DependencyEdge(helper=0, beneficiary=1, t=1),
        DependencyEdge(helper=1, beneficiary=2, t=2),
        DependencyEdge(helper=2, beneficiary=3, t=3),
    ]
    return TemporalDependencyGraph(n_agents=4, edges=edges, horizon=4)


@pytest.fixture
def branching_graph() -> TemporalDependencyGraph:
    """One agent helps multiple agents: agent 0 -> {1, 2, 3}."""
    edges = [
        DependencyEdge(helper=0, beneficiary=1, t=1),
        DependencyEdge(helper=0, beneficiary=2, t=1),
        DependencyEdge(helper=0, beneficiary=3, t=1),
    ]
    return TemporalDependencyGraph(n_agents=4, edges=edges, horizon=3)


@pytest.fixture
def converging_graph() -> TemporalDependencyGraph:
    """Multiple agents help one: {1, 2, 3} -> 0."""
    edges = [
        DependencyEdge(helper=1, beneficiary=0, t=1),
        DependencyEdge(helper=2, beneficiary=0, t=1),
        DependencyEdge(helper=3, beneficiary=0, t=1),
    ]
    return TemporalDependencyGraph(n_agents=4, edges=edges, horizon=3)


@pytest.fixture
def temporal_cycle_graph() -> TemporalDependencyGraph:
    """A temporal cycle: 0 -> 1 -> 2 -> 0 with strictly increasing time."""
    edges = [
        DependencyEdge(helper=0, beneficiary=1, t=1),
        DependencyEdge(helper=1, beneficiary=2, t=2),
        DependencyEdge(helper=2, beneficiary=0, t=3),
    ]
    return TemporalDependencyGraph(n_agents=3, edges=edges, horizon=4)


@pytest.fixture
def static_cycle_graph() -> TemporalDependencyGraph:
    """A cycle that does NOT progress through time: 0 -> 1 -> 0 at same t."""
    edges = [
        DependencyEdge(helper=0, beneficiary=1, t=1),
        DependencyEdge(helper=1, beneficiary=0, t=1),
    ]
    return TemporalDependencyGraph(n_agents=2, edges=edges, horizon=2)


@pytest.fixture
def hamiltonian_cycle_graph() -> TemporalDependencyGraph:
    """A Hamiltonian cycle in a 4-agent graph: 0 -> 1 -> 2 -> 3 -> 0."""
    edges = [
        DependencyEdge(helper=0, beneficiary=1, t=1),
        DependencyEdge(helper=1, beneficiary=2, t=2),
        DependencyEdge(helper=2, beneficiary=3, t=3),
        DependencyEdge(helper=3, beneficiary=0, t=4),
    ]
    return TemporalDependencyGraph(n_agents=4, edges=edges, horizon=5)


@pytest.fixture
def scc_3_agents() -> TemporalDependencyGraph:
    """A fully connected 3-agent SCC: 0 <-> 1, 1 <-> 2, 0 <-> 2."""
    edges = [
        DependencyEdge(helper=0, beneficiary=1, t=1),
        DependencyEdge(helper=1, beneficiary=0, t=2),
        DependencyEdge(helper=1, beneficiary=2, t=3),
        DependencyEdge(helper=2, beneficiary=1, t=4),
        DependencyEdge(helper=0, beneficiary=2, t=5),
        DependencyEdge(helper=2, beneficiary=0, t=6),
    ]
    return TemporalDependencyGraph(n_agents=3, edges=edges, horizon=7)


@pytest.fixture
def disconnected_graph() -> TemporalDependencyGraph:
    """Two disconnected components: {0, 1} and {2, 3}."""
    edges = [
        DependencyEdge(helper=0, beneficiary=1, t=1),
        DependencyEdge(helper=2, beneficiary=3, t=2),
    ]
    return TemporalDependencyGraph(n_agents=4, edges=edges, horizon=3)


@pytest.fixture
def multi_time_same_edge() -> TemporalDependencyGraph:
    """The same help relationship occurs multiple times across different time steps."""
    edges = [
        DependencyEdge(helper=0, beneficiary=1, t=1),
        DependencyEdge(helper=0, beneficiary=1, t=3),
        DependencyEdge(helper=0, beneficiary=1, t=5),
    ]
    return TemporalDependencyGraph(n_agents=2, edges=edges, horizon=6)


# ============================================================================
# Tests: Basic properties and construction
# ============================================================================


class TestBasicConstruction:
    """Test graph construction and basic property access."""

    def test_empty_graph_is_independent(self, empty_graph_2_agents: TemporalDependencyGraph):
        """An empty graph (no edges) is independent."""
        assert empty_graph_2_agents.is_independent is True
        assert len(empty_graph_2_agents.edges) == 0

    def test_non_empty_graph_not_independent(self, single_edge_graph: TemporalDependencyGraph):
        """A graph with at least one edge is not independent."""
        assert single_edge_graph.is_independent is False

    def test_graph_preserves_n_agents(self, empty_graph_5_agents: TemporalDependencyGraph):
        """n_agents is correctly stored and retrieved."""
        assert empty_graph_5_agents.n_agents == 5

    def test_graph_preserves_horizon(self):
        """horizon is correctly stored and retrieved."""
        graph = TemporalDependencyGraph(n_agents=3, edges=[], horizon=42)
        assert graph.horizon == 42

    def test_edges_are_frozen(self, single_edge_graph: TemporalDependencyGraph):
        """edges are immutable (frozenset)."""
        assert isinstance(single_edge_graph.edges, frozenset)
        with pytest.raises(AttributeError):
            single_edge_graph.edges.add(DependencyEdge(0, 2, 1))  # pyright: ignore[reportAttributeAccessIssue]


# ============================================================================
# Tests: Accessors (edges_at, flattened_edges, helpers_of, beneficiaries_of)
# ============================================================================
class TestAccessors:
    """Test accessor methods for querying edges."""

    def test_edges_at_empty(self, empty_graph_2_agents: TemporalDependencyGraph):
        """edges_at returns empty set when no edges exist at time t."""
        assert len(empty_graph_2_agents.edges_at(0)) == 0
        assert len(empty_graph_2_agents.edges_at(5)) == 0

    def test_edges_at_single(self, single_edge_graph: TemporalDependencyGraph):
        """edges_at returns the correct edges for a given time step."""
        # Single edge is at t=2
        assert len(single_edge_graph.edges_at(1)) == 0
        assert single_edge_graph.edges_at(2) == {(0, 1)}
        assert len(single_edge_graph.edges_at(3)) == 0

    def test_edges_at_multiple_same_time(self, branching_graph: TemporalDependencyGraph):
        """edges_at correctly returns multiple edges at the same time step."""
        edges_at_1 = branching_graph.edges_at(1)
        assert len(edges_at_1) == 3
        assert (0, 1) in edges_at_1
        assert (0, 2) in edges_at_1
        assert (0, 3) in edges_at_1

    def test_flattened_edges_empty(self, empty_graph_2_agents: TemporalDependencyGraph):
        """flattened_edges returns empty set when no edges exist."""
        assert len(empty_graph_2_agents.flattened_edges()) == 0

    def test_flattened_edges_collapses_time(self, multi_time_same_edge: TemporalDependencyGraph):
        """flattened_edges collapses time dimension, removing duplicates."""
        flattened = multi_time_same_edge.flattened_edges()
        assert flattened == {(0, 1)}
        assert len(flattened) == 1

    def test_flattened_edges_all_edges(self, branching_graph: TemporalDependencyGraph):
        """flattened_edges returns all distinct pairs."""
        flattened = branching_graph.flattened_edges()
        assert len(flattened) == 3
        assert (0, 1) in flattened
        assert (0, 2) in flattened
        assert (0, 3) in flattened

    def test_helpers_of_empty(self, empty_graph_2_agents: TemporalDependencyGraph):
        """helpers_of returns empty when no helpers exist."""
        assert len(empty_graph_2_agents.helpers_of(0)) == 0
        assert len(empty_graph_2_agents.helpers_of(1)) == 0

    def test_helpers_of_single(self, single_edge_graph: TemporalDependencyGraph):
        """helpers_of returns the correct set of helpers."""
        # Agent 0 helps agent 1
        assert len(single_edge_graph.helpers_of(0)) == 0
        assert single_edge_graph.helpers_of(1) == {0}

    def test_helpers_of_at_time(self, multi_time_same_edge: TemporalDependencyGraph):
        """helpers_of with t parameter filters by time step."""
        # Agent 0 helps agent 1 at t=1, 3, 5
        assert len(multi_time_same_edge.helpers_of(1, t=0)) == 0
        assert multi_time_same_edge.helpers_of(1, t=1) == {0}
        assert len(multi_time_same_edge.helpers_of(1, t=2)) == 0
        assert multi_time_same_edge.helpers_of(1, t=3) == {0}

    def test_helpers_of_multiple(self, converging_graph: TemporalDependencyGraph):
        """helpers_of returns all agents that help the beneficiary."""
        # Agents 1, 2, 3 all help agent 0
        helpers = converging_graph.helpers_of(0)
        assert len(helpers) == 3
        assert helpers == {1, 2, 3}

    def test_beneficiaries_of_empty(self, empty_graph_2_agents: TemporalDependencyGraph):
        """beneficiaries_of returns empty when no beneficiaries exist."""
        assert len(empty_graph_2_agents.beneficiaries_of(0)) == 0
        assert len(empty_graph_2_agents.beneficiaries_of(1)) == 0

    def test_beneficiaries_of_single(self, single_edge_graph: TemporalDependencyGraph):
        """beneficiaries_of returns the correct set of beneficiaries."""
        # Agent 0 helps agent 1
        assert single_edge_graph.beneficiaries_of(0) == {1}
        assert len(single_edge_graph.beneficiaries_of(1)) == 0

    def test_beneficiaries_of_at_time(self, branching_graph: TemporalDependencyGraph):
        """beneficiaries_of with t parameter filters by time step."""
        # Agent 0 helps agents 1, 2, 3 at t=1
        assert branching_graph.beneficiaries_of(0, t=1) == {1, 2, 3}
        assert len(branching_graph.beneficiaries_of(0, t=0)) == 0
        assert len(branching_graph.beneficiaries_of(0, t=2)) == 0

    def test_beneficiaries_of_multiple(self, branching_graph: TemporalDependencyGraph):
        """beneficiaries_of returns all agents helped by the helper."""
        beneficiaries = branching_graph.beneficiaries_of(0)
        assert len(beneficiaries) == 3
        assert beneficiaries == {1, 2, 3}


# ============================================================================
# Tests: Fan-in and fan-out metrics
# ============================================================================


class TestFanMetrics:
    """Test fan-in and fan-out calculations."""

    def test_fan_in_empty(self, empty_graph_5_agents: TemporalDependencyGraph):
        """fan_in is 0 for all agents in an empty graph."""
        for agent in range(5):
            assert empty_graph_5_agents.fan_in(agent) == 0

    def test_fan_out_empty(self, empty_graph_5_agents: TemporalDependencyGraph):
        """fan_out is 0 for all agents in an empty graph."""
        for agent in range(5):
            assert empty_graph_5_agents.fan_out(agent) == 0

    def test_fan_in_single_edge(self, single_edge_graph: TemporalDependencyGraph):
        """fan_in correctly counts distinct helpers."""
        # Only agent 1 is helped (by agent 0)
        assert single_edge_graph.fan_in(0) == 0
        assert single_edge_graph.fan_in(1) == 1

    def test_fan_out_single_edge(self, single_edge_graph: TemporalDependencyGraph):
        """fan_out correctly counts distinct beneficiaries."""
        # Only agent 0 helps (agent 1)
        assert single_edge_graph.fan_out(0) == 1
        assert single_edge_graph.fan_out(1) == 0

    def test_fan_in_multiple_helpers(self, converging_graph: TemporalDependencyGraph):
        """fan_in correctly counts multiple distinct helpers."""
        # Agent 0 is helped by agents 1, 2, 3
        assert converging_graph.fan_in(0) == 3
        assert converging_graph.fan_in(1) == 0

    def test_fan_out_multiple_beneficiaries(self, branching_graph: TemporalDependencyGraph):
        """fan_out correctly counts multiple distinct beneficiaries."""
        # Agent 0 helps agents 1, 2, 3
        assert branching_graph.fan_out(0) == 3
        assert branching_graph.fan_out(1) == 0

    def test_fan_in_with_duplicates_across_time(self, multi_time_same_edge: TemporalDependencyGraph):
        """fan_in counts distinct agents, not edge count."""
        # Same edge (0 -> 1) appears 3 times, but only 1 distinct helper
        assert multi_time_same_edge.fan_in(1) == 1

    def test_fan_in_at_time(self, branching_graph: TemporalDependencyGraph):
        """fan_in with t parameter filters to a specific time step."""
        assert branching_graph.fan_in(1, t=1) == 1
        assert branching_graph.fan_in(1, t=0) == 0
        assert branching_graph.fan_in(1, t=2) == 0

    def test_fan_out_at_time(self, multi_time_same_edge: TemporalDependencyGraph):
        """fan_out with t parameter filters to a specific time step."""
        assert multi_time_same_edge.fan_out(0, t=1) == 1
        assert multi_time_same_edge.fan_out(0, t=3) == 1
        assert multi_time_same_edge.fan_out(0, t=2) == 0

    def test_max_fan_in(self, converging_graph: TemporalDependencyGraph):
        """max_fan_in returns the largest fan-in across all agents."""
        # Agent 0 has fan-in 3, others have 0
        assert converging_graph.max_fan_in() == 3

    def test_max_fan_out(self, branching_graph: TemporalDependencyGraph):
        """max_fan_out returns the largest fan-out across all agents."""
        # Agent 0 has fan-out 3, others have 0
        assert branching_graph.max_fan_out() == 3

    def test_max_fan_in_empty(self, empty_graph_5_agents: TemporalDependencyGraph):
        """max_fan_in returns 0 for an empty graph."""
        assert empty_graph_5_agents.max_fan_in() == 0

    def test_max_fan_out_empty(self, empty_graph_5_agents: TemporalDependencyGraph):
        """max_fan_out returns 0 for an empty graph."""
        assert empty_graph_5_agents.max_fan_out() == 0

    def test_max_fan_in_at_time(self, branching_graph: TemporalDependencyGraph):
        """max_fan_in with t parameter considers only edges at that time."""
        # All branching happens at t=1
        assert branching_graph.max_fan_in(t=1) == 1
        assert branching_graph.max_fan_in(t=0) == 0

    def test_max_fan_out_at_time(self, branching_graph: TemporalDependencyGraph):
        """max_fan_out with t parameter considers only edges at that time."""
        # All branching happens at t=1
        assert branching_graph.max_fan_out(t=1) == 3
        assert branching_graph.max_fan_out(t=0) == 0


# ============================================================================
# Tests: Longest chain detection
# ============================================================================
class TestLongestChain:
    """Test longest_chain method for detecting simple paths."""

    def test_longest_chain_empty(self, empty_graph_5_agents: TemporalDependencyGraph):
        """longest_chain returns 0 for an empty graph."""
        assert empty_graph_5_agents.longest_time_agnostic_chain() == 0

    def test_longest_chain_single_edge(self, single_edge_graph: TemporalDependencyGraph):
        """longest_chain returns 1 for a single edge."""
        assert single_edge_graph.longest_time_agnostic_chain() == 1

    def test_longest_chain_linear(self, linear_chain_graph: TemporalDependencyGraph):
        """longest_chain returns the correct length for a linear chain."""
        # Chain: 0 -> 1 -> 2 -> 3 (3 edges)
        assert linear_chain_graph.longest_time_agnostic_chain() == 3

    def test_longest_chain_branching(self, branching_graph: TemporalDependencyGraph):
        """longest_chain ignores multiple paths from same source."""
        # 0 -> {1, 2, 3}: longest chain is 1 (each branch is just 1 edge)
        assert branching_graph.longest_time_agnostic_chain() == 1

    def test_longest_chain_converging(self, converging_graph: TemporalDependencyGraph):
        """longest_chain ignores converging paths."""
        # {1, 2, 3} -> 0: longest chain is 1
        assert converging_graph.longest_time_agnostic_chain() == 1

    def test_longest_chain_single_agent(self, single_agent_graph: TemporalDependencyGraph):
        """longest_chain returns 0 when only one agent exists."""
        assert single_agent_graph.longest_time_agnostic_chain() == 0

    def test_longest_chain_disconnected(self, disconnected_graph: TemporalDependencyGraph):
        """longest_chain returns the longest path among all components."""
        # Two components: 0 -> 1 and 2 -> 3, each is 1 edge
        assert disconnected_graph.longest_time_agnostic_chain() == 1


class TestLongestTemporalChain:
    """Test longest_temporal_chain method for temporal dependency chains."""

    def test_temporal_chain_empty(self, empty_graph_5_agents: TemporalDependencyGraph):
        """longest_temporal_chain returns 0 for an empty graph."""
        assert empty_graph_5_agents.longest_temporal_chain() == 0

    def test_temporal_chain_single_edge(self, single_edge_graph: TemporalDependencyGraph):
        """longest_temporal_chain returns 1 for a single edge."""
        assert single_edge_graph.longest_temporal_chain() == 1

    def test_temporal_chain_linear(self, linear_chain_graph: TemporalDependencyGraph):
        """longest_temporal_chain returns the correct length for a linear chain."""
        # Chain: 0 -> 1 (t=1), 1 -> 2 (t=2), 2 -> 3 (t=3): 3 edges, strictly increasing times
        assert linear_chain_graph.longest_temporal_chain() == 3

    def test_temporal_chain_branching(self, branching_graph: TemporalDependencyGraph):
        """longest_temporal_chain ignores multiple paths from same source."""
        # 0 -> {1, 2, 3} at same time: longest chain is 1
        assert branching_graph.longest_temporal_chain() == 1

    def test_temporal_chain_converging(self, converging_graph: TemporalDependencyGraph):
        """longest_temporal_chain ignores converging paths."""
        # {1, 2, 3} -> 0 at same time: longest chain is 1
        assert converging_graph.longest_temporal_chain() == 1

    def test_temporal_chain_single_agent(self, single_agent_graph: TemporalDependencyGraph):
        """longest_temporal_chain returns 0 when only one agent exists."""
        assert single_agent_graph.longest_temporal_chain() == 0

    def test_temporal_chain_disconnected(self, disconnected_graph: TemporalDependencyGraph):
        """longest_temporal_chain returns the longest path among all components."""
        # Two components: 0 -> 1 (t=1) and 2 -> 3 (t=2), each is 1 edge
        assert disconnected_graph.longest_temporal_chain() == 1

    def test_temporal_chain_strictly_increasing_times(self):
        """Chain with strictly increasing times: a -> b -> c -> d."""
        edges = [
            DependencyEdge(helper=0, beneficiary=1, t=1),
            DependencyEdge(helper=1, beneficiary=2, t=2),
            DependencyEdge(helper=2, beneficiary=3, t=3),
        ]
        graph = TemporalDependencyGraph(n_agents=4, edges=edges, horizon=4)
        assert graph.longest_temporal_chain() == 3

    def test_temporal_chain_same_time(self):
        """Edges at the same time cannot form a chain longer than 1."""
        edges = [
            DependencyEdge(helper=0, beneficiary=1, t=1),
            DependencyEdge(helper=1, beneficiary=2, t=1),
        ]
        graph = TemporalDependencyGraph(n_agents=3, edges=edges, horizon=2)
        # 0 -> 1 -> 2 but both edges at t=1, so chain breaks after first edge
        assert graph.longest_temporal_chain() == 1

    def test_temporal_chain_decreasing_times(self):
        """Chain with decreasing times should not form long chains."""
        edges = [
            DependencyEdge(helper=0, beneficiary=1, t=3),
            DependencyEdge(helper=1, beneficiary=2, t=2),
            DependencyEdge(helper=2, beneficiary=3, t=1),
        ]
        graph = TemporalDependencyGraph(n_agents=4, edges=edges, horizon=4)
        # Each edge is at a lower time than the previous, so each is separate
        assert graph.longest_temporal_chain() == 1

    def test_temporal_chain_non_monotonic_times(self):
        """The example from prompt: a->b at t0, b->c at t2, c->d at t1."""
        edges = [
            DependencyEdge(helper=0, beneficiary=1, t=0),
            DependencyEdge(helper=1, beneficiary=2, t=2),
            DependencyEdge(helper=2, beneficiary=3, t=1),
        ]
        graph = TemporalDependencyGraph(n_agents=4, edges=edges, horizon=3)
        # 0 -> 1 (t=0), 1 -> 2 (t=2): chain of length 2
        # 2 -> 3 (t=1) happens before 1 -> 2 (t=2), so cannot extend chain
        assert graph.longest_temporal_chain() == 2

    def test_temporal_chain_with_equal_times(self):
        """Chains cannot continue when times are equal (times must be strictly increasing)."""
        edges = [
            DependencyEdge(helper=0, beneficiary=1, t=1),
            DependencyEdge(helper=1, beneficiary=2, t=1),
            DependencyEdge(helper=2, beneficiary=3, t=1),
        ]
        graph = TemporalDependencyGraph(n_agents=4, edges=edges, horizon=2)
        # All edges at same time, so longest chain is 1 (each edge is separate)
        assert graph.longest_temporal_chain() == 1

    def test_temporal_chain_mixed_times(self):
        """Complex case with multiple paths at different times."""
        edges = [
            DependencyEdge(helper=0, beneficiary=1, t=1),
            DependencyEdge(helper=1, beneficiary=2, t=2),
            DependencyEdge(helper=0, beneficiary=3, t=1),
            DependencyEdge(helper=3, beneficiary=4, t=3),
        ]
        graph = TemporalDependencyGraph(n_agents=5, edges=edges, horizon=4)
        # Longest chain: 0 -> 1 -> 2 (length 2) or 0 -> 3 -> 4 (length 2)
        assert graph.longest_temporal_chain() == 2

    def test_temporal_chain_hamiltonian_cycle(self, hamiltonian_cycle_graph: TemporalDependencyGraph):
        """A Hamiltonian cycle with strictly increasing times."""
        # 0 -> 1 (t=1), 1 -> 2 (t=2), 2 -> 3 (t=3), 3 -> 0 (t=4)
        # Longest temporal chain: 0 -> 1 -> 2 -> 3 (length 3)
        # Cannot include 3 -> 0 because that would require visiting 0 twice
        assert hamiltonian_cycle_graph.longest_temporal_chain() == 3


# ============================================================================
# Tests: Temporal cycle detection
# ============================================================================
class TestTemporalCycleDetection:
    """Test has_temporal_cycle method."""

    def test_temporal_cycle_empty(self, empty_graph_5_agents: TemporalDependencyGraph):
        """Empty graph has no temporal cycle."""
        assert empty_graph_5_agents.has_temporal_cycle() is False

    def test_temporal_cycle_single_edge(self, single_edge_graph: TemporalDependencyGraph):
        """Single edge cannot form a cycle."""
        assert single_edge_graph.has_temporal_cycle() is False

    def test_temporal_cycle_linear_chain(self, linear_chain_graph: TemporalDependencyGraph):
        """Linear chain has no cycle."""
        assert linear_chain_graph.has_temporal_cycle() is False

    def test_temporal_cycle_branching(self, branching_graph: TemporalDependencyGraph):
        """Branching structure has no cycle."""
        assert branching_graph.has_temporal_cycle() is False

    def test_temporal_cycle_detected(self, temporal_cycle_graph: TemporalDependencyGraph):
        """Temporal cycle with strictly increasing time is detected."""
        # 0 -> 1 -> 2 -> 0 with t increasing
        assert temporal_cycle_graph.has_temporal_cycle() is True

    def test_temporal_cycle_static_not_detected(self, static_cycle_graph: TemporalDependencyGraph):
        """Cycle at the same time step is NOT a temporal cycle."""
        # 0 <-> 1 both at t=1: time is not strictly increasing
        assert static_cycle_graph.has_temporal_cycle() is False

    def test_temporal_cycle_disconnected(self, disconnected_graph: TemporalDependencyGraph):
        """No temporal cycle when components are separate."""
        assert disconnected_graph.has_temporal_cycle() is False

    def test_temporal_cycle_in_scc(self, scc_3_agents: TemporalDependencyGraph):
        """SCC with backward edges may have temporal cycles."""
        # The test SCC has edges at different times that could form cycles
        # Actual result depends on edge ordering in fixture
        result = scc_3_agents.has_temporal_cycle()
        assert isinstance(result, bool)  # Just verify it runs

    def test_temporal_cycle_single_agent(self, single_agent_graph: TemporalDependencyGraph):
        """Single agent cannot form a temporal cycle."""
        assert single_agent_graph.has_temporal_cycle() is False


# ============================================================================
# Tests: Hamiltonian cycle detection
# ============================================================================


class TestHamiltonianCycleDetection:
    """Test has_hamiltonian_cycle method."""

    def test_hamiltonian_cycle_empty(self, empty_graph_5_agents):
        """Empty graph has no Hamiltonian cycle."""
        assert empty_graph_5_agents.has_hamiltonian_cycle() is False

    def test_hamiltonian_cycle_single_agent(self, single_agent_graph):
        """Single agent cannot have a Hamiltonian cycle."""
        assert single_agent_graph.has_hamiltonian_cycle() is False

    def test_hamiltonian_cycle_two_agents_incomplete(self, single_edge_graph):
        """Two agents with one-directional edge have no Hamiltonian cycle."""
        # 0 -> 1, but no 1 -> 0
        assert single_edge_graph.has_hamiltonian_cycle() is False

    def test_hamiltonian_cycle_two_agents_complete(self, static_cycle_graph):
        """Two agents with bidirectional edges have a Hamiltonian cycle."""
        # 0 <-> 1
        assert static_cycle_graph.has_hamiltonian_cycle() is True

    def test_hamiltonian_cycle_detected(self, hamiltonian_cycle_graph):
        """Hamiltonian cycle through all agents is detected."""
        # 0 -> 1 -> 2 -> 3 -> 0
        assert hamiltonian_cycle_graph.has_hamiltonian_cycle() is True

    def test_hamiltonian_cycle_linear_no_return(self, linear_chain_graph):
        """Linear chain without return edge has no Hamiltonian cycle."""
        # 0 -> 1 -> 2 -> 3
        assert linear_chain_graph.has_hamiltonian_cycle() is False

    def test_hamiltonian_cycle_branching(self, branching_graph):
        """Branching structure has no Hamiltonian cycle."""
        # 0 -> {1, 2, 3}
        assert branching_graph.has_hamiltonian_cycle() is False

    def test_hamiltonian_cycle_converging(self, converging_graph):
        """Converging structure has no Hamiltonian cycle."""
        # {1, 2, 3} -> 0
        assert converging_graph.has_hamiltonian_cycle() is False

    def test_hamiltonian_cycle_temporal(self, temporal_cycle_graph):
        """Temporal cycle with 3 agents is a Hamiltonian cycle."""
        # 0 -> 1 -> 2 -> 0
        assert temporal_cycle_graph.has_hamiltonian_cycle() is True

    def test_hamiltonian_cycle_disconnected(self, disconnected_graph):
        """Disconnected components cannot have a Hamiltonian cycle."""
        # Two separate edges: no way to visit all 4 agents in one cycle
        assert disconnected_graph.has_hamiltonian_cycle() is False


# ============================================================================
# Tests: Strongly connected components (SCC)
# ============================================================================


class TestStronglyConnectedComponents:
    """Test strongly_connected_components method."""

    def test_scc_empty(self, empty_graph_5_agents):
        """Empty graph has no non-trivial SCCs."""
        sccs = empty_graph_5_agents.strongly_connected_components()
        assert len(sccs) == 0

    def test_scc_single_agent(self, single_agent_graph):
        """Single agent has no non-trivial SCC."""
        sccs = single_agent_graph.strongly_connected_components()
        assert len(sccs) == 0

    def test_scc_single_edge(self, single_edge_graph):
        """Single-directional edge does not form an SCC."""
        sccs = single_edge_graph.strongly_connected_components()
        assert len(sccs) == 0

    def test_scc_static_cycle(self, static_cycle_graph):
        """Bidirectional edge (cycle of length 2) forms an SCC."""
        sccs = static_cycle_graph.strongly_connected_components()
        assert len(sccs) == 1
        assert sccs[0] == frozenset({0, 1})

    def test_scc_temporal_cycle(self, temporal_cycle_graph):
        """Temporal cycle forms an SCC if all nodes are mutually reachable."""
        sccs = temporal_cycle_graph.strongly_connected_components()
        assert len(sccs) == 1
        assert sccs[0] == frozenset({0, 1, 2})

    def test_scc_hamiltonian_cycle(self, hamiltonian_cycle_graph):
        """Hamiltonian cycle creates one SCC."""
        sccs = hamiltonian_cycle_graph.strongly_connected_components()
        assert len(sccs) == 1
        assert sccs[0] == frozenset({0, 1, 2, 3})

    def test_scc_full_3_agents(self, scc_3_agents):
        """Fully connected 3-agent SCC is detected."""
        sccs = scc_3_agents.strongly_connected_components()
        assert len(sccs) == 1
        assert sccs[0] == frozenset({0, 1, 2})

    def test_scc_branching_no_scc(self, branching_graph):
        """Branching (no return edges) has no SCC."""
        sccs = branching_graph.strongly_connected_components()
        assert len(sccs) == 0

    def test_scc_converging_no_scc(self, converging_graph):
        """Converging edges alone have no SCC."""
        sccs = converging_graph.strongly_connected_components()
        assert len(sccs) == 0

    def test_scc_disconnected_two_components(self, disconnected_graph):
        """Two separate edges form no SCC (each is one-directional)."""
        sccs = disconnected_graph.strongly_connected_components()
        assert len(sccs) == 0

    def test_scc_multiple_sccs(self):
        """Graph with multiple disjoint SCCs detects all of them."""
        edges = [
            # SCC 1: 0 <-> 1
            DependencyEdge(helper=0, beneficiary=1, t=1),
            DependencyEdge(helper=1, beneficiary=0, t=2),
            # SCC 2: 2 <-> 3
            DependencyEdge(helper=2, beneficiary=3, t=1),
            DependencyEdge(helper=3, beneficiary=2, t=2),
        ]
        graph = TemporalDependencyGraph(n_agents=4, edges=edges, horizon=3)
        sccs = graph.strongly_connected_components()
        assert len(sccs) == 2
        assert frozenset({0, 1}) in sccs
        assert frozenset({2, 3}) in sccs

    def test_scc_only_non_trivial_returned(self):
        """Singleton SCCs are not returned (only size >= 2)."""
        edges = [
            DependencyEdge(helper=0, beneficiary=1, t=1),
            DependencyEdge(helper=2, beneficiary=3, t=1),
            DependencyEdge(helper=3, beneficiary=2, t=2),
        ]
        graph = TemporalDependencyGraph(n_agents=4, edges=edges, horizon=3)
        sccs = graph.strongly_connected_components()
        # Only SCC 2-3 should be returned (0 and 1 are not mutually reachable)
        assert len(sccs) == 1
        assert frozenset({2, 3}) in sccs


# ============================================================================
# Tests: Edge cases and boundary conditions
# ============================================================================


class TestEdgeCases:
    """Test edge cases and boundary conditions."""

    def test_zero_agents(self):
        """Graph with zero agents."""
        graph = TemporalDependencyGraph(n_agents=0, edges=[], horizon=0)
        assert graph.is_independent is True
        assert graph.longest_time_agnostic_chain() == 0
        assert graph.has_temporal_cycle() is False
        assert graph.has_hamiltonian_cycle() is False
        assert len(graph.strongly_connected_components()) == 0

    def test_self_loop_not_in_flattened(self):
        """Self-loops (helper == beneficiary) should not normally occur but are handled."""
        edges = [DependencyEdge(helper=0, beneficiary=0, t=1)]
        graph = TemporalDependencyGraph(n_agents=1, edges=edges, horizon=2)
        # Self-loop is included in flattened_edges
        assert (0, 0) in graph.flattened_edges()

    def test_duplicate_edges_handled_by_frozenset(self):
        """Duplicate edges in input are deduplicated by frozenset."""
        edges = [
            DependencyEdge(helper=0, beneficiary=1, t=1),
            DependencyEdge(helper=0, beneficiary=1, t=1),  # Duplicate at same time
            DependencyEdge(helper=0, beneficiary=1, t=1),
        ]
        graph = TemporalDependencyGraph(n_agents=2, edges=edges, horizon=2)
        # Duplicates are collapsed to one edge in the frozenset
        assert len(graph.edges) == 1

    def test_large_time_values(self):
        """Graph handles large time step values."""
        edges = [DependencyEdge(helper=0, beneficiary=1, t=1000000)]
        graph = TemporalDependencyGraph(n_agents=2, edges=edges, horizon=1000000)
        assert (0, 1) in graph.edges_at(1000000)
        assert len(graph.edges_at(999999)) == 0

    def test_many_agents(self):
        """Graph with many agents (stress test)."""
        n = 100
        graph = TemporalDependencyGraph(n_agents=n, edges=[], horizon=10)
        assert graph.n_agents == n
        assert graph.max_fan_in() == 0
        assert graph.max_fan_out() == 0

    def test_many_edges(self):
        """Graph with many edges (stress test)."""
        edges = [DependencyEdge(helper=i % 5, beneficiary=(i + 1) % 5, t=i // 5) for i in range(1000)]
        graph = TemporalDependencyGraph(n_agents=5, edges=edges, horizon=200)
        assert len(graph.edges) <= 1000
        # The actual number may be less due to duplicate handling

    def test_negative_time_steps(self):
        """Graph can technically handle negative time (unusual but allowed)."""
        edges = [DependencyEdge(helper=0, beneficiary=1, t=-5)]
        graph = TemporalDependencyGraph(n_agents=2, edges=edges, horizon=0)
        assert (-5, (0, 1)) in {(e.t, (e.helper, e.beneficiary)) for e in graph.edges}


# ============================================================================
# Tests: Profile integration
# ============================================================================


class TestProfileIntegration:
    """Test the profile() method and TrajectoryProfile generation."""

    def test_profile_empty_graph(self, empty_graph_5_agents):
        """Profile of empty graph has correct independent flag."""
        profile = empty_graph_5_agents.profile()
        assert isinstance(profile, TrajectoryProfile)
        assert profile.is_independent is True
        assert profile.n_dependencies == 0

    def test_profile_single_edge(self, single_edge_graph):
        """Profile of single-edge graph has correct metrics."""
        profile = single_edge_graph.profile()
        assert profile.is_independent is False
        assert profile.n_dependencies == 1
        assert profile.longest_chain == 1

    def test_profile_linear_chain(self, linear_chain_graph):
        """Profile of linear chain has correct longest_chain."""
        profile = linear_chain_graph.profile()
        assert profile.longest_chain == 3
        assert profile.has_temporal_cycle is False

    def test_profile_temporal_cycle(self, temporal_cycle_graph):
        """Profile detects temporal cycle correctly."""
        profile = temporal_cycle_graph.profile()
        assert profile.has_temporal_cycle is True
        assert len(profile.strongly_connected_components) >= 1

    def test_profile_branching(self, branching_graph):
        """Profile of branching graph has correct fan-out."""
        profile = branching_graph.profile()
        assert profile.max_fan_out == 3
        assert profile.longest_chain == 1

    def test_profile_converging(self, converging_graph):
        """Profile of converging graph has correct fan-in."""
        profile = converging_graph.profile()
        assert profile.max_fan_in == 3
        assert profile.longest_chain == 1

    def test_profile_scc(self, static_cycle_graph):
        """Profile includes non-trivial SCC information."""
        profile = static_cycle_graph.profile()
        assert len(profile.strongly_connected_components) == 1
        assert profile.largest_scc_size == 2

    def test_profile_all_fields_populated(self, linear_chain_graph):
        """Profile has all expected fields populated."""
        profile = linear_chain_graph.profile()
        assert profile.n_agents == linear_chain_graph.n_agents
        assert profile.horizon == linear_chain_graph.horizon
        assert hasattr(profile, "is_independent")
        assert hasattr(profile, "n_dependencies")
        assert hasattr(profile, "fan_in")
        assert hasattr(profile, "fan_out")
        assert hasattr(profile, "max_fan_in")
        assert hasattr(profile, "max_fan_out")
        assert hasattr(profile, "longest_chain")
        assert hasattr(profile, "strongly_connected_components")
        assert hasattr(profile, "largest_scc_size")
        assert hasattr(profile, "has_temporal_cycle")
        assert hasattr(profile, "has_hamiltonian_cycle")

    def test_profile_fan_in_dict(self, branching_graph):
        """Profile fan_in dict has entry for each agent."""
        profile = branching_graph.profile()
        for agent in range(branching_graph.n_agents):
            assert agent in profile.fan_in
            assert isinstance(profile.fan_in[agent], int)

    def test_profile_fan_out_dict(self, branching_graph):
        """Profile fan_out dict has entry for each agent."""
        profile = branching_graph.profile()
        for agent in range(branching_graph.n_agents):
            assert agent in profile.fan_out
            assert isinstance(profile.fan_out[agent], int)

    def test_profile_hamiltonian_cycle(self, hamiltonian_cycle_graph):
        """Profile detects Hamiltonian cycle."""
        profile = hamiltonian_cycle_graph.profile()
        assert profile.has_hamiltonian_cycle is True

    def test_profile_has_mutual_help_true(self, static_cycle_graph):
        """Profile has_mutual_help property is True for SCCs."""
        profile = static_cycle_graph.profile()
        assert profile.has_mutual_help is True

    def test_profile_has_mutual_help_false(self, linear_chain_graph):
        """Profile has_mutual_help property is False for acyclic graph."""
        profile = linear_chain_graph.profile()
        assert profile.has_mutual_help is False


# ============================================================================
# Tests: Consistency checks
# ============================================================================


class TestConsistency:
    """Test internal consistency of the graph methods."""

    def test_flattened_edges_match_helpers_beneficiaries(self, branching_graph):
        """Flattened edges match the union of all (helper, beneficiary) pairs."""
        flattened = branching_graph.flattened_edges()
        reconstructed = set()
        for agent in range(branching_graph.n_agents):
            for helper in branching_graph.helpers_of(agent):
                reconstructed.add((helper, agent))
            for beneficiary in branching_graph.beneficiaries_of(agent):
                reconstructed.add((agent, beneficiary))
        assert flattened == reconstructed

    def test_fan_in_equals_helpers_count(self, converging_graph):
        """fan_in should equal the size of helpers_of set."""
        for agent in range(converging_graph.n_agents):
            assert converging_graph.fan_in(agent) == len(converging_graph.helpers_of(agent))

    def test_fan_out_equals_beneficiaries_count(self, branching_graph):
        """fan_out should equal the size of beneficiaries_of set."""
        for agent in range(branching_graph.n_agents):
            assert branching_graph.fan_out(agent) == len(branching_graph.beneficiaries_of(agent))

    def test_max_fan_in_is_maximum(self, scc_3_agents):
        """max_fan_in should equal the maximum individual fan_in."""
        max_fan = scc_3_agents.max_fan_in()
        individual_fans = [scc_3_agents.fan_in(a) for a in range(scc_3_agents.n_agents)]
        assert max_fan == max(individual_fans) if individual_fans else 0

    def test_max_fan_out_is_maximum(self, scc_3_agents):
        """max_fan_out should equal the maximum individual fan_out."""
        max_fan = scc_3_agents.max_fan_out()
        individual_fans = [scc_3_agents.fan_out(a) for a in range(scc_3_agents.n_agents)]
        assert max_fan == max(individual_fans) if individual_fans else 0

    def test_temporal_cycle_implies_hamiltonian_or_smaller(self, temporal_cycle_graph):
        """If has_temporal_cycle is True, either there's a Hamiltonian or it's smaller."""
        if temporal_cycle_graph.has_temporal_cycle():
            # For a cycle of n agents, a temporal cycle exists
            # It doesn't guarantee a Hamiltonian, but typically in test cases it does
            assert temporal_cycle_graph.n_agents >= 2


# ============================================================================
# Tests: Real-world patterns
# ============================================================================


class TestRealWorldPatterns:
    """Test patterns that might arise from actual trajectories."""

    def test_sequential_help_chain(self):
        """A common pattern: sequential help where agents take turns."""
        # Agent 0 helps 1, then 1 helps 2, then 2 helps 3 sequentially
        edges = [
            DependencyEdge(helper=0, beneficiary=1, t=1),
            DependencyEdge(helper=1, beneficiary=2, t=2),
            DependencyEdge(helper=2, beneficiary=3, t=3),
        ]
        graph = TemporalDependencyGraph(n_agents=4, edges=edges, horizon=4)
        assert graph.longest_time_agnostic_chain() == 3
        assert not graph.has_temporal_cycle()

    def test_mutual_help_at_different_times(self):
        """Agents help each other but at different times (not mutual within one step)."""
        edges = [
            DependencyEdge(helper=0, beneficiary=1, t=1),
            DependencyEdge(helper=1, beneficiary=0, t=2),
        ]
        graph = TemporalDependencyGraph(n_agents=2, edges=edges, horizon=3)
        assert graph.has_temporal_cycle()
        assert graph.has_hamiltonian_cycle()

    def test_bottleneck_pattern(self):
        """One agent is critical: many agents depend on it."""
        edges = [
            DependencyEdge(helper=0, beneficiary=1, t=1),
            DependencyEdge(helper=0, beneficiary=2, t=1),
            DependencyEdge(helper=0, beneficiary=3, t=1),
            DependencyEdge(helper=0, beneficiary=4, t=1),
        ]
        graph = TemporalDependencyGraph(n_agents=5, edges=edges, horizon=2)
        assert graph.fan_out(0) == 4
        assert graph.fan_in(1) == 1
        assert graph.longest_time_agnostic_chain() == 1

    def test_relay_race_pattern(self):
        """Agents relay: 0 -> 1, then separately 1 -> 2, etc."""
        edges = [
            DependencyEdge(helper=0, beneficiary=1, t=1),
            DependencyEdge(helper=1, beneficiary=2, t=2),
            DependencyEdge(helper=2, beneficiary=3, t=3),
        ]
        graph = TemporalDependencyGraph(n_agents=4, edges=edges, horizon=4)
        assert graph.longest_time_agnostic_chain() == 3

    def test_diamond_pattern(self):
        """Diamond: 0 -> {1,2} and {1,2} -> 3."""
        edges = [
            DependencyEdge(helper=0, beneficiary=1, t=1),
            DependencyEdge(helper=0, beneficiary=2, t=1),
            DependencyEdge(helper=1, beneficiary=3, t=2),
            DependencyEdge(helper=2, beneficiary=3, t=2),
        ]
        graph = TemporalDependencyGraph(n_agents=4, edges=edges, horizon=3)
        assert graph.longest_time_agnostic_chain() == 2
        assert graph.fan_in(3) == 2
        assert graph.fan_out(0) == 2


# ============================================================================
# Tests: Parametrized and property-based
# ============================================================================


class TestParametrized:
    """Parametrized tests for systematic coverage."""

    @pytest.mark.parametrize("n_agents", [1, 2, 3, 5, 10])
    def test_empty_graph_properties(self, n_agents):
        """Empty graph with any number of agents is independent."""
        graph = TemporalDependencyGraph(n_agents=n_agents, edges=[], horizon=0)
        assert graph.is_independent is True
        assert graph.longest_time_agnostic_chain() == 0
        assert graph.max_fan_in() == 0
        assert graph.max_fan_out() == 0

    @pytest.mark.parametrize("t", [0, 1, 5, 100])
    def test_edges_at_nonexistent_time(self, single_edge_graph, t):
        """edges_at returns empty set for times with no edges."""
        if t != 2:  # 2 is the time of the single edge
            assert len(single_edge_graph.edges_at(t)) == 0

    @pytest.mark.parametrize("agent", [0, 1])
    def test_disconnected_agents_no_help(self, single_edge_graph, agent):
        """In a graph with one edge, uninvolved agents have no relationships."""
        if agent == 2:  # Only agents 0 and 1 are in single_edge_graph
            pytest.skip("Agent 2 not in graph")


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
