"""Tests for the cooperation analyser (`lle.cooperation`).

The analyser replays a trajectory, builds the temporal helper graph, and exposes
structural cooperation properties. Each test below uses a hand-crafted world and
a hand-driven trajectory whose helper edges are known by construction.

Beam reminder: a vertical laser of colour ``c`` placed at the top of a column
projects its beam downwards. Only agent ``c`` can stand on the beam (it blocks
it); any other agent standing on a tile *below* the blocker survives because the
beam is off there.
"""

from __future__ import annotations

from copy import deepcopy

from lle import Action, World
from lle.cooperation import (
    CooperationProfile,
    DependencyEdge,
    TemporalDependencyGraph,
    analyse_cooperation,
    detect_dependencies,
)

N, S, E, W, St = Action.NORTH, Action.SOUTH, Action.EAST, Action.WEST, Action.STAY


# ---------------------------------------------------------------------------
# Worlds used across tests
# ---------------------------------------------------------------------------
def _two_beam_world() -> World:
    """Colour-0 beam in column 1, colour-1 beam in column 2 (adjacent)."""
    return World(
        "\n".join(
            [
                ".  L0S L1S X",
                "S0 .   .   .",
                ".  .   .   .",
                "S1 .   .   X",
            ]
        )
    )


def _single_beam_world() -> World:
    """Colour-0 beam in column 1, three agents that can stand below the blocker."""
    return World(
        "\n".join(
            [
                ".  L0S .",
                "S0 .   X",
                "S1 .   X",
                "S2 .   X",
            ]
        )
    )


# ---------------------------------------------------------------------------
# detect_dependencies
# ---------------------------------------------------------------------------
def test_detect_dependencies_blocker_and_beneficiary():
    world = _single_beam_world()
    world.reset()
    # No one on the beam yet.
    assert detect_dependencies(world) == set()
    # Agent 0 blocks its own beam, agent 1 steps onto a downstream tile.
    world.step([E, St, St])
    assert detect_dependencies(world) == set()  # blocker alone, nobody helped yet
    world.step([St, E, St])
    assert detect_dependencies(world) == {(0, 1)}


def test_detect_dependencies_requires_a_blocker():
    """An agent on a beam with no same-colour blocker is not a beneficiary (it dies)."""
    world = _single_beam_world()
    world.reset()
    # Agent 1 walks onto the beam while agent 0 does NOT block: agent 1 dies,
    # so it is not registered as an occupant and no edge is produced.
    world.step([St, E, St])
    assert detect_dependencies(world) == set()
    assert world.get_state().agents_alive[1] is False


# ---------------------------------------------------------------------------
# Independent trajectory
# ---------------------------------------------------------------------------
def test_independent_world_has_no_dependencies():
    world = World("S0 . X\nS1 . X")
    graph = analyse_cooperation(world, [[E, E], [E, E]])
    assert graph.is_independent
    assert graph.flattened_edges() == set()

    profile = graph.profile()
    assert profile.is_independent
    assert profile.n_dependencies == 0
    assert profile.max_fan_in == 0
    assert profile.max_fan_out == 0
    assert profile.longest_chain == 0
    assert profile.strongly_connected_components == []
    assert profile.largest_scc_size == 0
    assert not profile.has_temporal_cycle
    assert not profile.has_hamiltonian_cycle


# ---------------------------------------------------------------------------
# Asymmetric help (0 -> 1)
# ---------------------------------------------------------------------------
def test_single_asymmetric_dependency():
    world = _two_beam_world()
    trajectory = [[E, St], [St, E], [St, N]]
    graph = analyse_cooperation(world, trajectory)

    assert not graph.is_independent
    assert graph.flattened_edges() == {(0, 1)}
    assert graph.edges_at(2) == {(0, 1)}
    assert graph.helpers_of(1) == {0}
    assert graph.beneficiaries_of(0) == {1}
    assert graph.fan_in(1) == 1
    assert graph.fan_out(0) == 1
    assert graph.fan_in(0) == 0

    profile = graph.profile()
    assert profile.longest_chain == 1
    assert profile.max_fan_in == 1
    assert profile.max_fan_out == 1
    assert profile.strongly_connected_components == []
    assert not profile.has_temporal_cycle
    assert not profile.has_hamiltonian_cycle


# ---------------------------------------------------------------------------
# Mutual help (0 -> 1 and 1 -> 0 at different times)
# ---------------------------------------------------------------------------
def test_mutual_dependency_forms_a_cycle():
    world = _two_beam_world()
    # 0 helps 1 early (t=2,3); then 1 blocks the colour-1 beam and helps 0 (t=7).
    trajectory = [[E, St], [St, E], [St, N], [St, E], [S, St], [S, St], [E, St]]
    graph = analyse_cooperation(world, trajectory)

    assert graph.flattened_edges() == {(0, 1), (1, 0)}
    assert graph.edges_at(2) == {(0, 1)}
    assert graph.edges_at(7) == {(1, 0)}

    profile = graph.profile()
    assert profile.max_fan_in == 1
    assert profile.max_fan_out == 1
    # Flattened: the only simple paths are single edges.
    assert profile.longest_chain == 1
    assert profile.strongly_connected_components == [frozenset({0, 1})]
    assert profile.largest_scc_size == 2
    assert profile.has_temporal_cycle
    assert profile.has_hamiltonian_cycle


def test_simultaneous_mutual_help_is_not_a_temporal_cycle():
    """Mutual help that happens at the same instant is not a temporal cycle.

    A temporal cycle requires edges that progress strictly through time; two
    mutual edges sharing one time step cannot be ordered into a forward loop.
    """
    graph = TemporalDependencyGraph(
        n_agents=2,
        edges=[DependencyEdge(0, 1, 4), DependencyEdge(1, 0, 4)],
        horizon=4,
    )
    assert not graph.has_temporal_cycle()
    # Flattened, it is still a strongly connected component.
    assert graph.strongly_connected_components() == [frozenset({0, 1})]
    assert graph.has_hamiltonian_cycle()


# ---------------------------------------------------------------------------
# Chain (0 -> 1 -> 2)
# ---------------------------------------------------------------------------
def test_chain_dependency():
    world = World(
        "\n".join(
            [
                ".   L0S .   L1S X",
                "S0  .   .   .   X",
                "S1  .   .   .   X",
                ".   .   .   .   .",
                ".   .   .   .   S2",
            ]
        )
    )
    trajectory = [
        [E, St, St],  # agent 0 blocks colour-0 beam
        [St, E, St],  # agent 1 protected on colour-0 beam -> 0 -> 1
        [St, E, St],  # agent 1 moves towards the colour-1 beam
        [St, E, N],   # agent 1 blocks colour-1 beam; agent 2 approaches
        [St, St, W],  # agent 2 protected on colour-1 beam -> 1 -> 2
    ]
    graph = analyse_cooperation(world, trajectory)

    assert graph.flattened_edges() == {(0, 1), (1, 2)}
    profile = graph.profile()
    assert profile.longest_chain == 2
    assert profile.max_fan_in == 1
    assert profile.max_fan_out == 1
    assert profile.strongly_connected_components == []
    assert not profile.has_temporal_cycle
    assert not profile.has_hamiltonian_cycle
    # Agent 1 is both a beneficiary (of 0) and a helper (of 2).
    assert graph.helpers_of(1) == {0}
    assert graph.beneficiaries_of(1) == {2}


# ---------------------------------------------------------------------------
# Fan-out (0 helps 1 and 2 simultaneously)
# ---------------------------------------------------------------------------
def test_simultaneous_fan_out():
    world = _single_beam_world()
    trajectory = [[E, St, St], [St, E, St], [St, St, E]]
    graph = analyse_cooperation(world, trajectory)

    # At the final state, both beneficiaries are downstream of agent 0.
    assert graph.edges_at(3) == {(0, 1), (0, 2)}
    assert graph.fan_out(0, t=3) == 2
    assert graph.fan_out(0, t=2) == 1
    assert graph.fan_out(0) == 2  # time-agnostic

    profile = graph.profile()
    assert profile.max_fan_out == 2
    assert profile.max_fan_in == 1  # each beneficiary helped only by agent 0
    assert profile.longest_chain == 1
    assert not profile.has_temporal_cycle


# ---------------------------------------------------------------------------
# Hamiltonian cycle over three agents
# ---------------------------------------------------------------------------
def test_hamiltonian_cycle_three_agents():
    graph = TemporalDependencyGraph(
        n_agents=3,
        edges=[DependencyEdge(0, 1, 0), DependencyEdge(1, 2, 1), DependencyEdge(2, 0, 2)],
        horizon=2,
    )
    assert graph.has_hamiltonian_cycle()
    assert graph.has_temporal_cycle()
    assert graph.strongly_connected_components() == [frozenset({0, 1, 2})]
    # The longest simple path uses all three edges around the ring but cannot
    # revisit a node, so it spans two edges.
    assert graph.longest_chain() == 2


def test_no_hamiltonian_cycle_when_one_agent_isolated():
    graph = TemporalDependencyGraph(
        n_agents=3,
        edges=[DependencyEdge(0, 1, 0), DependencyEdge(1, 0, 1)],
        horizon=1,
    )
    assert not graph.has_hamiltonian_cycle()  # agent 2 is unreachable
    assert graph.strongly_connected_components() == [frozenset({0, 1})]


# ---------------------------------------------------------------------------
# Graph-level queries
# ---------------------------------------------------------------------------
def test_time_specific_queries():
    graph = TemporalDependencyGraph(
        n_agents=3,
        edges=[DependencyEdge(0, 2, 1), DependencyEdge(1, 2, 1), DependencyEdge(0, 2, 4)],
        horizon=4,
    )
    # At t=1, agent 2 is helped by both 0 and 1.
    assert graph.helpers_of(2, t=1) == {0, 1}
    assert graph.fan_in(2, t=1) == 2
    # Time-agnostic fan-in is still 2 (only agents 0 and 1 ever help it).
    assert graph.fan_in(2) == 2
    assert graph.edges_at(4) == {(0, 2)}
    assert graph.max_fan_in(t=1) == 2
    assert graph.max_fan_out(t=1) == 1


# ---------------------------------------------------------------------------
# Side-effect freedom and input shapes
# ---------------------------------------------------------------------------
def test_analyse_does_not_mutate_input_world():
    world = _two_beam_world()
    world.reset()
    before = deepcopy(world.get_state())
    analyse_cooperation(world, [[E, St], [St, E]])
    assert world.get_state() == before


def test_single_agent_accepts_bare_action():
    world = World("S0 . G X")
    graph = analyse_cooperation(world, [E, E, E])
    assert graph.is_independent
    assert graph.n_agents == 1
    assert graph.horizon == 3


def test_profile_from_graph_matches_object():
    world = _two_beam_world()
    graph = analyse_cooperation(world, [[E, St], [St, E]])
    profile = graph.profile()
    assert isinstance(profile, CooperationProfile)
    assert profile.n_agents == 2
    assert profile.horizon == 2
    assert profile.fan_out[0] == graph.fan_out(0)
    assert profile.fan_in[1] == graph.fan_in(1)
