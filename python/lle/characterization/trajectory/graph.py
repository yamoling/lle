"""The temporal helper graph and the structural properties extracted from it.

A *dependency* (or *helper*) edge ``helper -> beneficiary`` at time step ``t``
means that, at time ``t``, ``helper`` blocks a laser of its own colour while
``beneficiary`` stands on a tile of that beam without dying (the beam is blocked
for the beneficiary).  See `lle.cooperation.analyser` for how these edges are
detected from a trajectory.

Two complementary views of the same edge set are offered:

* the **temporal** view keeps the time stamp on every edge, which matters for
  chains and cycles that must progress through time;
* the **flattened** (time-agnostic) view collapses every edge onto the pair
  ``(helper, beneficiary)``, which is the right view for fan-in/fan-out summaries
  and strongly connected components.
"""

from __future__ import annotations

from collections import defaultdict
from collections.abc import Iterable
from dataclasses import dataclass
from typing import TYPE_CHECKING

from lle.types import AgentId

if TYPE_CHECKING:
    from .profile import TrajectoryProfile


@dataclass(frozen=True)
class DependencyEdge:
    """A single ``helper -> beneficiary`` relationship at one time step."""

    helper: AgentId
    """The agent that blocks its own laser."""
    beneficiary: AgentId
    """The agent that is protected by the blocked beam."""
    t: int
    """The time step (state index) at which the help occurs."""


class TemporalDependencyGraph:
    """The time-wise agent dependency graph of a single trajectory.

    Edges are directed from the *helper* to the *beneficiary*, so that the
    out-degree of a vertex is its fan-out (how many agents it helps) and the
    in-degree is its fan-in (by how many agents it is helped).
    """

    def __init__(self, n_agents: int, edges: Iterable[DependencyEdge], horizon: int):
        self.n_agents = n_agents
        """The number of agents in the world."""
        self.horizon = horizon
        """The index of the last state, i.e. the number of actions in the trajectory."""
        self._edges = frozenset(edges)

    # ------------------------------------------------------------------
    # Basic accessors
    # ------------------------------------------------------------------
    @property
    def edges(self) -> frozenset[DependencyEdge]:
        """All temporal dependency edges."""
        return self._edges

    @property
    def is_independent(self) -> bool:
        """Whether the trajectory contains no dependency at all."""
        return len(self._edges) == 0

    def edges_at(self, t: int) -> set[tuple[AgentId, AgentId]]:
        """The ``(helper, beneficiary)`` pairs active exactly at time step ``t``."""
        return {(e.helper, e.beneficiary) for e in self._edges if e.t == t}

    def flattened_edges(self) -> set[tuple[AgentId, AgentId]]:
        """The set of ``(helper, beneficiary)`` pairs across all time steps."""
        return {(e.helper, e.beneficiary) for e in self._edges}

    def helpers_of(self, beneficiary: AgentId, t: int | None = None) -> set[AgentId]:
        """The agents that help ``beneficiary`` (at time ``t`` if given, else ever)."""
        return {e.helper for e in self._edges if e.beneficiary == beneficiary and (t is None or e.t == t)}

    def beneficiaries_of(self, helper: AgentId, t: int | None = None) -> set[AgentId]:
        """The agents that ``helper`` helps (at time ``t`` if given, else ever)."""
        return {e.beneficiary for e in self._edges if e.helper == helper and (t is None or e.t == t)}

    # ------------------------------------------------------------------
    # Fan-in / fan-out
    # ------------------------------------------------------------------
    def fan_in(self, beneficiary: AgentId, t: int | None = None) -> int:
        """How many distinct agents help ``beneficiary`` (at time ``t`` if given)."""
        return len(self.helpers_of(beneficiary, t))

    def fan_out(self, helper: AgentId, t: int | None = None) -> int:
        """How many distinct agents ``helper`` helps (at time ``t`` if given)."""
        return len(self.beneficiaries_of(helper, t))

    def max_fan_in(self, t: int | None = None) -> int:
        """The largest fan-in over all agents (at time ``t`` if given)."""
        return max((self.fan_in(a, t) for a in range(self.n_agents)), default=0)

    def max_fan_out(self, t: int | None = None) -> int:
        """The largest fan-out over all agents (at time ``t`` if given)."""
        return max((self.fan_out(a, t) for a in range(self.n_agents)), default=0)

    # ------------------------------------------------------------------
    # Chains
    # ------------------------------------------------------------------
    def longest_chain(self) -> int:
        """The number of edges in the longest help chain.

        A chain ``a -> b -> c -> ...`` is a simple directed path (no repeated
        agent) in the flattened graph.  The returned value counts edges, so a
        single help relationship has length ``1`` and ``a -> b -> c`` has length
        ``2``.  An independent graph has length ``0``.
        """
        adjacency = self._flattened_adjacency()

        def dfs(node: AgentId, visited: set[AgentId]) -> int:
            best = 0
            for nxt in adjacency[node]:
                if nxt in visited:
                    continue
                visited.add(nxt)
                best = max(best, 1 + dfs(nxt, visited))
                visited.remove(nxt)
            return best

        return max((dfs(start, {start}) for start in range(self.n_agents)), default=0)

    # ------------------------------------------------------------------
    # Cycles
    # ------------------------------------------------------------------
    def has_temporal_cycle(self) -> bool:
        """Whether a cycle exists whose edges progress strictly through time.

        A temporal cycle is a sequence ``v0 -> v1 -> ... -> v0`` whose successive
        edges have strictly increasing time stamps (e.g. ``a -> b`` at ``t``,
        ``b -> c`` at ``t + 1``, ``c -> a`` at ``t + 2``).  Such a cycle means
        that, over time, every agent on it relies on every other.
        """
        by_helper: dict[AgentId, list[tuple[AgentId, int]]] = defaultdict(list)
        for e in self._edges:
            by_helper[e.helper].append((e.beneficiary, e.t))

        for start in range(self.n_agents):
            memo: dict[tuple[AgentId, int], bool] = {}

            def can_return(node: AgentId, last_t: int, start: AgentId = start) -> bool:
                key = (node, last_t)
                cached = memo.get(key)
                if cached is not None:
                    return cached
                result = False
                for nxt, t in by_helper.get(node, ()):
                    if t <= last_t:
                        continue
                    if nxt == start or can_return(nxt, t):
                        result = True
                        break
                memo[key] = result
                return result

            if can_return(start, -1):
                return True
        return False

    def has_hamiltonian_cycle(self) -> bool:
        """Whether the flattened graph has a cycle visiting every agent exactly once."""
        if self.n_agents < 2:
            return False
        adjacency = self._flattened_adjacency()
        start = 0

        def dfs(node: AgentId, visited: set[AgentId]) -> bool:
            if len(visited) == self.n_agents:
                return start in adjacency[node]
            for nxt in adjacency[node]:
                if nxt not in visited:
                    visited.add(nxt)
                    if dfs(nxt, visited):
                        return True
                    visited.remove(nxt)
            return False

        return dfs(start, {start})

    # ------------------------------------------------------------------
    # Strongly connected components (flattened)
    # ------------------------------------------------------------------
    def strongly_connected_components(self) -> list[frozenset[AgentId]]:
        """The non-trivial strongly connected components of the flattened graph.

        Only components with at least two agents are returned: a singleton SCC
        carries no cooperation information (an agent never helps itself).  A
        returned component means every agent in it (transitively) helps and is
        helped by every other agent in it.
        """
        adjacency = self._flattened_adjacency()
        index_of: dict[AgentId, int] = {}
        low_link: dict[AgentId, int] = {}
        on_stack: set[AgentId] = set()
        stack: list[AgentId] = []
        counter = 0
        components: list[frozenset[AgentId]] = []

        def strong_connect(v: AgentId) -> None:
            nonlocal counter
            index_of[v] = counter
            low_link[v] = counter
            counter += 1
            stack.append(v)
            on_stack.add(v)
            for w in adjacency[v]:
                if w not in index_of:
                    strong_connect(w)
                    low_link[v] = min(low_link[v], low_link[w])
                elif w in on_stack:
                    low_link[v] = min(low_link[v], index_of[w])
            if low_link[v] == index_of[v]:
                component: set[AgentId] = set()
                while True:
                    w = stack.pop()
                    on_stack.discard(w)
                    component.add(w)
                    if w == v:
                        break
                if len(component) >= 2:
                    components.append(frozenset(component))

        for v in range(self.n_agents):
            if v not in index_of:
                strong_connect(v)
        return components

    # ------------------------------------------------------------------
    # Helpers
    # ------------------------------------------------------------------
    def _flattened_adjacency(self) -> dict[AgentId, set[AgentId]]:
        adjacency: dict[AgentId, set[AgentId]] = {a: set() for a in range(self.n_agents)}
        for helper, beneficiary in self.flattened_edges():
            adjacency[helper].add(beneficiary)
        return adjacency

    def profile(self) -> "TrajectoryProfile":
        """Summarise the graph into a `CooperationProfile`."""
        from .profile import TrajectoryProfile

        return TrajectoryProfile.from_graph(self)

    def __repr__(self) -> str:
        return f"TemporalDependencyGraph(n_agents={self.n_agents}, horizon={self.horizon}, n_edges={len(self._edges)})"
