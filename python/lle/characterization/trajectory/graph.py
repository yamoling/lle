"""The temporal helper graph and the structural properties extracted from it.

A *dependency* (or *helper*) edge`helper -> beneficiary` at time step`t`
means that, at time`t`,`helper` blocks a laser of its own colour while
`beneficiary` stands on a tile of that beam without dying (the beam is blocked
for the beneficiary).  See `lle.cooperation.analyser` for how these edges are
detected from a trajectory.
"""

from __future__ import annotations

from collections import defaultdict, deque
from collections.abc import Iterable
from dataclasses import dataclass
from typing import TYPE_CHECKING

from lle.types import AgentId

if TYPE_CHECKING:
    from .profile import TrajectoryProfile


@dataclass(frozen=True)
class DependencyEdge:
    """A single`helper -> beneficiary` relationship at one time step."""

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
    def edges(self):
        """All temporal dependency edges."""
        return self._edges

    @property
    def is_empty(self):
        """Whether the trajectory contains no edge at all."""
        return len(self._edges) == 0

    def edges_at(self, t: int) -> set[tuple[AgentId, AgentId]]:
        """The`(helper, beneficiary)` pairs active exactly at time step`t`."""
        return {(e.helper, e.beneficiary) for e in self._edges if e.t == t}

    def flattened_edges(self) -> set[tuple[AgentId, AgentId]]:
        """The set of`(helper, beneficiary)` pairs across all time steps."""
        return {(e.helper, e.beneficiary) for e in self._edges}

    def helpers_of(self, beneficiary: AgentId, t: int | None = None) -> set[AgentId]:
        """The agents that help`beneficiary` (at time`t` if given, else ever)."""
        return {e.helper for e in self._edges if e.beneficiary == beneficiary and (t is None or e.t == t)}

    def beneficiaries_of(self, helper: AgentId, t: int | None = None) -> set[AgentId]:
        """The agents that`helper` helps (at time`t` if given, else ever)."""
        return {e.beneficiary for e in self._edges if e.helper == helper and (t is None or e.t == t)}

    def asymmetric_edges(self) -> set[tuple[AgentId, AgentId]]:
        """Flattened help edges whose helper is never helped by any other agent."""
        edges = self.flattened_edges()
        helped_agents = {beneficiary for _, beneficiary in edges}
        return {(helper, beneficiary) for helper, beneficiary in edges if helper not in helped_agents}

    def has_asymmetric_edge(self) -> bool:
        """Whether some agent helps another agent without ever being helped itself."""
        return len(self.asymmetric_edges()) > 0

    # ------------------------------------------------------------------
    # Fan-in / fan-out
    # ------------------------------------------------------------------
    def fan_in(self, beneficiary: AgentId, t: int | None = None) -> int:
        """How many distinct agents help`beneficiary` (at time`t` if given)."""
        return len(self.helpers_of(beneficiary, t))

    def fan_out(self, helper: AgentId, t: int | None = None) -> int:
        """How many distinct agents`helper` helps (at time`t` if given)."""
        return len(self.beneficiaries_of(helper, t))

    def max_fan_in(self, t: int | None = None) -> int:
        """The largest fan-in over all agents (at time`t` if given)."""
        return max((self.fan_in(a, t) for a in range(self.n_agents)), default=0)

    def max_fan_out(self, t: int | None = None) -> int:
        """The largest fan-out over all agents (at time`t` if given)."""
        return max((self.fan_out(a, t) for a in range(self.n_agents)), default=0)

    def longest_walk(self) -> int | float:
        """Return the length of the longest non-decreasing-time help-edge walk.

        A chain is a directed walk of help edges whose timestamps never decrease. Agents and lasers
        are not resources: the same agent may be revisited and the same help edge may be used again
        at the same or a later time. If one time bucket contains a directed cycle, the longest walk
        is unbounded and this method returns `float("inf")`. It reports `0` when the longest finite
        walk has fewer than two edges (a single help edge is cooperation, but not a chain).

        # Examples
           - `a -> b` returns 1;
           - `a -> b -> c` returns `2`;
           - `a -> b -> c -> a` returns `3`;
           - `a -> b -> c -> d -> b` returns `4`;
           - `a -> b`, and `a -> c` returns `1`;
           - `a -> b -> a` returns `2`;
           - an independent graph returns `0`.
        """
        if not self._edges:
            return 0

        best_ending_at: dict[AgentId, int] = defaultdict(int)
        longest = 0

        sorted_edges = sorted(self._edges, key=lambda edge: edge.t)
        idx = 0
        while idx < len(sorted_edges):
            t = sorted_edges[idx].t
            bucket: list[DependencyEdge] = []
            while idx < len(sorted_edges) and sorted_edges[idx].t == t:
                bucket.append(sorted_edges[idx])
                idx += 1

            adjacency: dict[AgentId, set[AgentId]] = defaultdict(set)
            indegree: dict[AgentId, int] = defaultdict(int)
            nodes: set[AgentId] = set()
            for edge in bucket:
                nodes.add(edge.helper)
                nodes.add(edge.beneficiary)
                if edge.beneficiary not in adjacency[edge.helper]:
                    adjacency[edge.helper].add(edge.beneficiary)
                    indegree[edge.beneficiary] += 1
                    indegree.setdefault(edge.helper, indegree.get(edge.helper, 0))

            queue = deque(node for node in nodes if indegree[node] == 0)
            topo: list[AgentId] = []
            while queue:
                node = queue.popleft()
                topo.append(node)
                for nxt in adjacency.get(node, set()):
                    indegree[nxt] -= 1
                    if indegree[nxt] == 0:
                        queue.append(nxt)

            if len(topo) != len(nodes):
                return float("inf")

            depths = {node: best_ending_at[node] for node in nodes}
            for node in topo:
                for nxt in adjacency.get(node, set()):
                    length = depths[node] + 1
                    depths[nxt] = max(depths.get(nxt, 0), length)
                    longest = max(longest, length)

            for node, length in depths.items():
                best_ending_at[node] = max(best_ending_at[node], length)

        return longest if longest >= 2 else 0

    def longest_chain(self) -> int | float:
        """Alias for [`longest_walk`](TemporalDependencyGraph.longest_walk)."""
        return self.longest_walk()

    # ------------------------------------------------------------------
    # Cycles
    # ------------------------------------------------------------------
    def max_temporal_cycle_order(self, strict: bool = False) -> int:
        """Size of the largest simple directed cycle in the temporal graph with non-decreasing
        (or strictly increasing, if `strict=True`) timestamps, or 0 if no cycle exists.

        A temporal cycle of order`k` visits`k` distinct agents and returns to its start,
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

         A cycle is detected when there exist two agents`a` and`b` such that
        `a` helps`b` at time`t1` and`b` helps`a` at time`t2 > t1`.
         Same-timestep mutual edges (`t1 == t2`) are not counted because the
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

    def profile(self) -> "TrajectoryProfile":
        """Summarise the graph into a `TrajectoryProfile`."""
        from .profile import TrajectoryProfile

        return TrajectoryProfile(self)

    def __repr__(self) -> str:
        return f"TemporalDependencyGraph(n_agents={self.n_agents}, horizon={self.horizon}, n_edges={len(self._edges)})"
