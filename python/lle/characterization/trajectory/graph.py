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
    def is_empty(self):
        """Whether the trajectory contains no edge at all."""
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

    def longest_chain(self) -> int:
        """
        A chain ``(a, t0) -> (b, t1) -> (c, t2) -> ...`` is a temporal directed path whose
        edges progress strictly through time. A chain encodes the idea of transitivity of the
        cooperation: if a helps b and b helps c, then a also helps c indirectly.

        Agents may repeat only to close a temporal cycle back to the starting agent. In
        particular, ``a -> b -> a`` counts as a chain of length 2, while longer walks such as
        ``a -> b -> c -> a`` also count but stop when they return to their start.

        A chain must have a length of at least 2 edges, otherwise it is not a chain.

        # Returns
        The returned value counts the length of the longest chain in the graph.

        For instance:
           - a single help relationship returns 0;
           - `a -> b -> c` returns `2`;
           - `a -> b -> c -> a` returns `2`;
           - `a -> b -> c -> d` returns `3`;
           - `a -> b`, and `a -> c` returns `0`;
           - `a -> b -> a` returns `2`;
           - an independent graph returns `0`.
        """
        by_helper: dict[AgentId, list[tuple[AgentId, int]]] = defaultdict(list)
        for e in self._edges:
            by_helper[e.helper].append((e.beneficiary, e.t))

        def dfs(start: AgentId, node: AgentId, visited: set[AgentId], last_t: int) -> int:
            best = 0
            for nxt, t in by_helper.get(node, []):
                if t < last_t:
                    continue
                if nxt == start and len(visited) >= 2:
                    best = max(best, 1)
                    continue
                if nxt in visited:
                    continue
                visited.add(nxt)
                best = max(best, 1 + dfs(start, nxt, visited, t))
                visited.remove(nxt)
            return best

        max_length = max((dfs(start, start, {start}, -1) for start in range(self.n_agents)), default=0)
        if max_length < 2:
            return 0
        return max_length

    # ------------------------------------------------------------------
    # Cycles
    # ------------------------------------------------------------------
    def max_temporal_cycle_order(self, strict: bool = False) -> int:
        """Size of the largest simple directed cycle in the temporal graph with non-decreasing
        (or strictly increasing, if ``strict=True``) timestamps, or 0 if no cycle exists.

        A temporal cycle of order ``k`` visits ``k`` distinct agents and returns to its start,
        with each edge's timestamp ≥ the previous one (non-strict) or > (strict).
        """
        by_helper: dict[AgentId, list[tuple[AgentId, int]]] = defaultdict(list)
        for e in self._edges:
            by_helper[e.helper].append((e.beneficiary, e.t))

        best = 0

        def dfs(start: AgentId, node: AgentId, visited: set[AgentId], last_t: int) -> None:
            nonlocal best
            for nxt, t in by_helper.get(node, []):
                if strict:
                    if t <= last_t:
                        continue
                else:
                    if t < last_t:
                        continue
                if nxt == start and len(visited) >= 2:
                    best = max(best, len(visited))
                    continue
                if nxt in visited:
                    continue
                visited.add(nxt)
                dfs(start, nxt, visited, t)
                visited.remove(nxt)

        for start in range(self.n_agents):
            dfs(start, start, {start}, -1)

        return best

    def has_cycle(self) -> bool:
        """Whether a mutual-help cycle exists with strictly increasing time.

        A cycle is detected when there exist two agents ``a`` and ``b`` such that
        ``a`` helps ``b`` at time ``t1`` and ``b`` helps ``a`` at time ``t2 > t1``.
        Same-timestep mutual edges (``t1 == t2``) are not counted because the
        strictly-increasing requirement is not satisfied.
        """
        by_helper: dict[AgentId, set[tuple[AgentId, int]]] = defaultdict(set)
        for e in self._edges:
            by_helper[e.helper].add((e.beneficiary, e.t))

        for e in self._edges:
            for nxt, t_reverse in by_helper.get(e.beneficiary, set()):
                if nxt == e.helper and t_reverse > e.t:
                    return True
        return False

    def has_time_agnostic_cycle(self) -> bool:
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
        """Summarise the graph into a `TrajectoryProfile`."""
        from .profile import TrajectoryProfile

        return TrajectoryProfile(self)

    def __repr__(self) -> str:
        return f"TemporalDependencyGraph(n_agents={self.n_agents}, horizon={self.horizon}, n_edges={len(self._edges)})"
