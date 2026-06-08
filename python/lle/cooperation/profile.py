"""A flat summary of the structural properties of a temporal helper graph.

`CooperationProfile` collects the scalar and small-collection properties that
characterise the cooperation of a trajectory.  The properties are deliberately
*incomparable*: a long chain, a wide fan-out and a large strongly connected
component describe different cooperation structures and must not be ordered
against each other.
"""

from __future__ import annotations

from dataclasses import dataclass

from ..types import AgentId
from .graph import TemporalDependencyGraph


@dataclass(frozen=True)
class CooperationProfile:
    """Structural cooperation properties extracted from a `TemporalDependencyGraph`."""

    n_agents: int
    """The number of agents in the world."""
    horizon: int
    """The number of actions in the analysed trajectory."""
    is_independent: bool
    """Whether the trajectory required no help at all."""
    n_dependencies: int
    """The number of distinct ``(helper, beneficiary)`` pairs across time."""
    fan_in: dict[AgentId, int]
    """Per agent, by how many distinct agents it is helped over the whole trajectory."""
    fan_out: dict[AgentId, int]
    """Per agent, how many distinct agents it helps over the whole trajectory."""
    max_fan_in: int
    """The largest fan-in over all agents."""
    max_fan_out: int
    """The largest fan-out over all agents."""
    longest_chain: int
    """The number of edges in the longest help chain (simple path)."""
    strongly_connected_components: list[frozenset[AgentId]]
    """The non-trivial strongly connected components of the flattened graph."""
    largest_scc_size: int
    """The number of agents in the largest strongly connected component (``0`` if none)."""
    has_temporal_cycle: bool
    """Whether a cycle progressing strictly through time exists."""
    has_hamiltonian_cycle: bool
    """Whether the flattened graph has a cycle visiting every agent."""

    @staticmethod
    def from_graph(graph: TemporalDependencyGraph) -> "CooperationProfile":
        """Compute every property of ``graph`` in a single pass."""
        sccs = graph.strongly_connected_components()
        return CooperationProfile(
            n_agents=graph.n_agents,
            horizon=graph.horizon,
            is_independent=graph.is_independent,
            n_dependencies=len(graph.flattened_edges()),
            fan_in={a: graph.fan_in(a) for a in range(graph.n_agents)},
            fan_out={a: graph.fan_out(a) for a in range(graph.n_agents)},
            max_fan_in=graph.max_fan_in(),
            max_fan_out=graph.max_fan_out(),
            longest_chain=graph.longest_chain(),
            strongly_connected_components=sccs,
            largest_scc_size=max((len(c) for c in sccs), default=0),
            has_temporal_cycle=graph.has_temporal_cycle(),
            has_hamiltonian_cycle=graph.has_hamiltonian_cycle(),
        )
