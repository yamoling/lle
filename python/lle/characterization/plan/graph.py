from __future__ import annotations

from bisect import bisect_left, bisect_right
from collections.abc import Iterable
from dataclasses import dataclass
from functools import cache

from lle.types import AgentId
from lle.world import World

from .types import Plan


@dataclass(frozen=True)
class DependencyEdge:
    """A single `helper -> beneficiary` relationship at one time step."""

    helper: AgentId
    """The agent that blocks its own laser."""
    beneficiary: AgentId
    """The agent that is protected by the blocked beam."""
    t: int
    """The time step (state index) at which the help occurs."""


@dataclass(frozen=True)
class TimeLayer:
    """All beneficiaries helped by one helper at one time step."""

    t: int
    helper: AgentId
    beneficiaries: tuple[AgentId, ...]

    def edges(self):
        """
        Yield the concrete dependency edges represented by this layer.
        """
        for beneficiary in self.beneficiaries:
            yield DependencyEdge(self.helper, beneficiary, self.t)


@dataclass(frozen=True)
class AgentVertex:
    """A temporal vertex schedule for one helper agent."""

    agent_id: AgentId
    layers: tuple[TimeLayer, ...] = ()


class TemporalCooperationGraph:
    """
    A Temporal Cooperation Graph (TCG) represents the help dependencies between agents
    over time.

    Layers are grouped by helper, then by time, and beneficiaries in each
    layer are sorted to make trail tie-breaking deterministic.
    """

    def __init__(self, edges: Iterable[DependencyEdge]):
        edges = set(e for e in edges if e.helper != e.beneficiary)
        self._edges = tuple(sorted(edges, key=lambda edge: (edge.t, edge.helper, edge.beneficiary)))

        by_helper_time: dict[AgentId, dict[int, set[AgentId]]] = {}
        by_time_helper: dict[int, dict[AgentId, set[AgentId]]] = {}
        edge_ids_by_helper: dict[AgentId, list[int]] = {}

        for edge_id, edge in enumerate(self._edges):
            by_helper_time.setdefault(edge.helper, {}).setdefault(edge.t, set()).add(edge.beneficiary)
            by_time_helper.setdefault(edge.t, {}).setdefault(edge.helper, set()).add(edge.beneficiary)
            edge_ids_by_helper.setdefault(edge.helper, []).append(edge_id)

        agent_ids = sorted(set(e.helper for e in edges) | set(e.beneficiary for e in edges))
        self.vertices = {
            agent_id: AgentVertex(
                agent_id,
                tuple(
                    TimeLayer(t, agent_id, tuple(sorted(beneficiaries)))
                    for t, beneficiaries in sorted(by_helper_time.get(agent_id, {}).items())
                ),
            )
            for agent_id in agent_ids
        }
        self._by_time = {
            t: {helper: tuple(sorted(beneficiaries)) for helper, beneficiaries in sorted(by_helper.items())}
            for t, by_helper in sorted(by_time_helper.items())
        }
        self._edge_ids_by_helper = {helper: tuple(edge_ids) for helper, edge_ids in edge_ids_by_helper.items()}
        self._edge_times_by_helper = {
            helper: tuple(self._edges[edge_id].t for edge_id in edge_ids) for helper, edge_ids in self._edge_ids_by_helper.items()
        }

    @staticmethod
    def from_plan(plan: Plan, world: World, *, reset: bool = True):
        """
        Build a temporal dependency graph by replaying `plan` on the `world`.

        ## Parameters
        - `plan`: The sequence of joint actions to replay. Each element is either a single
            `Action` (for a single-agent world) or a sequence of one `Action` per agent.
        - `world`: The world to analyse. It **is** mutated (reset).
        - `reset`: Whether to reset the copied world before replaying. Keep the default unless the
            trajectory is meant to continue from the world's current state.
        """
        from .analyser import detect_dependencies

        if reset:
            world.reset()
        edges = [DependencyEdge(helper, beneficiary, 0) for helper, beneficiary in detect_dependencies(world)]
        for t, joint_action in enumerate(plan, start=1):
            world.step(joint_action)
            for helper, beneficiary in detect_dependencies(world):
                edges.append(DependencyEdge(helper, beneficiary, t))
        return TemporalCooperationGraph(edges)

    @property
    def edges(self):
        """All temporal dependency edges."""
        return self._edges

    @property
    def n_vertices(self):
        return len(self.vertices)

    @property
    def is_empty(self):
        """Whether the trajectory contains no edge at all."""
        return len(self._edges) == 0

    def flattened_edges(self):
        """Return the set of `(helper, beneficiary)` pairs across all time steps."""
        return {(edge.helper, edge.beneficiary) for edge in self._edges}

    def max_distinct_helpers(self) -> int:
        """Return the greatest number of distinct helpers of one beneficiary."""
        helpers_by_beneficiary: dict[AgentId, set[AgentId]] = {}
        for helper, beneficiary in self.flattened_edges():
            helpers_by_beneficiary.setdefault(beneficiary, set()).add(helper)
        return max((len(helpers) for helpers in helpers_by_beneficiary.values()), default=0)

    def max_distinct_beneficiaries(self) -> int:
        """Return the greatest number of distinct beneficiaries of one helper."""
        beneficiaries_by_helper: dict[AgentId, set[AgentId]] = {}
        for helper, beneficiary in self.flattened_edges():
            beneficiaries_by_helper.setdefault(helper, set()).add(beneficiary)
        return max((len(beneficiaries) for beneficiaries in beneficiaries_by_helper.values()), default=0)

    def asymmetric_edges(self):
        """Return flattened help edges whose helper is never helped by any other agent."""
        edges = self.flattened_edges()
        helped_agents = {beneficiary for _, beneficiary in edges}
        return {(helper, beneficiary) for helper, beneficiary in edges if helper not in helped_agents}

    def has_asymmetric_edge(self):
        """Return whether some helper is never helped by another agent."""
        return len(self.asymmetric_edges()) > 0

    def _edge_ids_after(self, helper: AgentId, t: int, *, strict: bool = False):
        """
        Return outgoing edge ids for `helper` with time `>= t` (or `> t` if `strict`).
        """
        edge_ids = self._edge_ids_by_helper.get(helper, ())
        edge_times = self._edge_times_by_helper.get(helper, ())
        start = bisect_right(edge_times, t) if strict else bisect_left(edge_times, t)
        return edge_ids[start:]

    def longest_trail(self) -> list[DependencyEdge]:
        """
        Return the longest non-decreasing-time trail as a sequence of edges.

        A trail may revisit agents, but each temporal edge triple can appear at
        most once. The exact search state therefore includes the current agent,
        the minimum allowed time, and a bit mask of used edges that are still
        reusable at that time. Caching only by current agent would be unsound:
        after a same-time cycle, reaching the same agent with different edges
        already used can leave different suffixes available.

        This is exact, but longest-trail search is exponential in the worst case
        when same-time cycles are present. The used-edge mask is normalised by
        timestamp so edges that can no longer be reused do not fragment the
        memoization cache.
        """
        if len(self._edges) == 0:
            return []

        first_time = self._edges[0].t
        edge_ids_by_time: dict[int, list[int]] = {}
        for edge_id, edge in enumerate(self._edges):
            edge_ids_by_time.setdefault(edge.t, []).append(edge_id)

        reusable_mask_by_time: dict[int, int] = {}
        reusable_mask = 0
        for t in sorted(edge_ids_by_time, reverse=True):
            for edge_id in edge_ids_by_time[t]:
                reusable_mask |= 1 << edge_id
            reusable_mask_by_time[t] = reusable_mask

        @cache
        def best_suffix(current: AgentId, min_t: int, used_mask: int):
            """Return the best edge-id suffix from this exact trail state."""
            best: tuple[int, ...] = ()
            for edge_id in self._edge_ids_after(current, min_t):
                edge_bit = 1 << edge_id
                if used_mask & edge_bit:
                    continue

                edge = self._edges[edge_id]
                next_used_mask = (used_mask | edge_bit) & reusable_mask_by_time[edge.t]
                suffix = best_suffix(edge.beneficiary, edge.t, next_used_mask)
                candidate = (edge_id, *suffix)
                if len(candidate) > len(best):
                    best = candidate
            return best

        best_trail_ids: tuple[int, ...] = ()
        for helper in sorted(self._edge_ids_by_helper):
            candidate = best_suffix(helper, first_time, 0)
            if len(candidate) > len(best_trail_ids):
                best_trail_ids = candidate

        return [self._edges[edge_id] for edge_id in best_trail_ids]

    def longest_trail_length(self):
        """Return the number of edges in the longest temporal trail."""
        return len(self.longest_trail())

    @staticmethod
    def _max_closed_trail_length(order: int) -> int:
        """
        Return the irreducible closed-trail edge bound for an exact support order.
        """
        half_order = order // 2
        if order % 2 == 0:
            return half_order * (half_order + 1)
        return (half_order + 1) * (half_order + 1)

    def closed_trail_of_order(self, order: int) -> list[DependencyEdge]:
        """
        Return one non-decreasing temporal closed trail with exactly `order` agents.

        Agents and static help arcs may recur. A static arc may recur only at a
        later timestamp, so the search tracks arcs used in the current time
        layer and resets that set whenever time advances.
        """
        if order < 2 or order > len(self.vertices):
            return []

        max_depth = self._max_closed_trail_length(order)
        first_time = self._edges[0].t

        def dfs(
            anchor: AgentId,
            current: AgentId,
            min_t: int | None,
            visited_agents: frozenset[AgentId],
            current_time_arcs: frozenset[tuple[AgentId, AgentId]],
            depth: int,
            path: tuple[DependencyEdge, ...],
        ) -> tuple[DependencyEdge, ...] | None:
            """Find an exact-support closed trail below one chronological state."""
            if depth == max_depth:
                return None

            for edge in self._edge_ids_after(current, min_t if min_t is not None else first_time):
                candidate = self._edges[edge]
                if candidate.helper == candidate.beneficiary:
                    continue
                static_arc = (candidate.helper, candidate.beneficiary)
                if candidate.t == min_t:
                    if static_arc in current_time_arcs:
                        continue
                    next_time_arcs = current_time_arcs | {static_arc}
                else:
                    next_time_arcs = frozenset({static_arc})

                next_visited = visited_agents | {candidate.beneficiary}
                if len(next_visited) > order:
                    continue
                next_path = (*path, candidate)
                if candidate.beneficiary == anchor and len(next_visited) == order:
                    return next_path

                found = dfs(
                    anchor,
                    candidate.beneficiary,
                    candidate.t,
                    next_visited,
                    next_time_arcs,
                    depth + 1,
                    next_path,
                )
                if found is not None:
                    return found
            return None

        for anchor in self.vertices:
            found = dfs(anchor, anchor, None, frozenset({anchor}), frozenset(), 0, ())
            if found is not None:
                return list(found)
        return []

    def has_closed_trail_of_order(self, order: int) -> bool:
        return bool(self.closed_trail_of_order(order))

    def profile(self):
        """Summarise the graph into a `TrajectoryProfile`."""
        from .profile import PlanProfile

        return PlanProfile(self)

    @staticmethod
    def empty():
        return TemporalCooperationGraph([])
