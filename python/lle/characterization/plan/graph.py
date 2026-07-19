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
        self._sorted_edges = tuple(sorted(frozenset(edges), key=lambda edge: (edge.t, edge.helper, edge.beneficiary)))
        self._edges = frozenset(self._sorted_edges)

        by_helper_time: dict[AgentId, dict[int, set[AgentId]]] = {}
        by_time_helper: dict[int, dict[AgentId, set[AgentId]]] = {}
        edge_ids_by_helper: dict[AgentId, list[int]] = {}

        for edge_id, edge in enumerate(self._sorted_edges):
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
            helper: tuple(self._sorted_edges[edge_id].t for edge_id in edge_ids) for helper, edge_ids in self._edge_ids_by_helper.items()
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
    def is_empty(self):
        """Whether the trajectory contains no edge at all."""
        return len(self._edges) == 0

    def active_times(self):
        """
        Return all time steps that contain at least one dependency edge.
        """
        return tuple(self._by_time.keys())

    def layers_for(self, helper: AgentId):
        """Return the sorted temporal layers for `helper`."""
        return self.vertices[helper].layers

    def beneficiaries_at(self, helper: AgentId, t: int):
        """Return the beneficiaries helped by `helper` exactly at time `t`."""
        return self._by_time.get(t, {}).get(helper, ())

    def flattened_edges(self):
        """
        Return the set of `(helper, beneficiary)` pairs across all time steps.
        """
        return {(edge.helper, edge.beneficiary) for edge in self._sorted_edges}

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

    def outgoing_after(self, helper: AgentId, t: int, *, strict: bool = False):
        """
        Yield outgoing temporal edges for `helper` in ascending timestamp order.

        With the default `strict=False`, edges at the same timestamp are yielded.
        With `strict=True`, the next edge must occur later than `t`.
        """
        for edge_id in self._edge_ids_after(helper, t, strict=strict):
            yield self._sorted_edges[edge_id]

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

        @ai-generated
        """
        if len(self._sorted_edges) == 0:
            return []

        first_time = self._sorted_edges[0].t
        edge_ids_by_time: dict[int, list[int]] = {}
        for edge_id, edge in enumerate(self._sorted_edges):
            edge_ids_by_time.setdefault(edge.t, []).append(edge_id)

        reusable_mask_by_time: dict[int, int] = {}
        reusable_mask = 0
        for t in sorted(edge_ids_by_time, reverse=True):
            for edge_id in edge_ids_by_time[t]:
                reusable_mask |= 1 << edge_id
            reusable_mask_by_time[t] = reusable_mask

        @cache
        def best_suffix(current: AgentId, min_t: int, used_mask: int):
            """
            Return the best edge-id suffix from this exact trail state.

            @ai-generated
            """
            best: tuple[int, ...] = ()
            for edge_id in self._edge_ids_after(current, min_t):
                edge_bit = 1 << edge_id
                if used_mask & edge_bit:
                    continue

                edge = self._sorted_edges[edge_id]
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

        return [self._sorted_edges[edge_id] for edge_id in best_trail_ids]

    def longest_trail_length(self):
        """Return the number of edges in the longest temporal trail."""
        return len(self.longest_trail())

    def longest_cycle(self) -> list[DependencyEdge]:
        """
        Return one of the longest simple temporal cycles, or an empty list.

        Cycle edges follow non-decreasing timestamps. The returned
        cycle is represented by its edge sequence; the last edge's beneficiary
        is the first edge's helper.

        @ai-generated
        """
        if len(self._sorted_edges) == 0:
            return []

        first_time = self._sorted_edges[0].t
        first_edge_floor = first_time - 1
        best: list[DependencyEdge] = []

        def dfs(
            start: AgentId,
            current: AgentId,
            min_t: int,
            visited_agents: set[AgentId],
            used_edges: set[DependencyEdge],
            path: list[DependencyEdge],
        ) -> None:
            """
            Explore simple temporal cycles rooted at `start`.

            @ai-generated
            """
            nonlocal best
            for edge in self.outgoing_after(current, min_t, strict=False):
                if edge in used_edges:
                    continue
                if edge.beneficiary == start and len(visited_agents) >= 2:
                    candidate = [*path, edge]
                    if len(candidate) > len(best):
                        best = candidate
                    continue
                if edge.beneficiary in visited_agents:
                    continue

                used_edges.add(edge)
                visited_agents.add(edge.beneficiary)
                path.append(edge)
                dfs(start, edge.beneficiary, edge.t, visited_agents, used_edges, path)
                path.pop()
                visited_agents.remove(edge.beneficiary)
                used_edges.remove(edge)

        for start in self.vertices:
            dfs(start, start, first_edge_floor, {start}, set(), [])

        return best

    @staticmethod
    def _max_closed_trail_length(order: int) -> int:
        """
        Return the irreducible closed-trail edge bound for an exact support order.

        @ai-generated
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

        @ai-generated
        """
        if order < 2 or order > len(self.vertices):
            return []

        max_depth = self._max_closed_trail_length(order)
        first_time = self._sorted_edges[0].t

        def dfs(
            anchor: AgentId,
            current: AgentId,
            min_t: int | None,
            visited_agents: frozenset[AgentId],
            current_time_arcs: frozenset[tuple[AgentId, AgentId]],
            depth: int,
            path: tuple[DependencyEdge, ...],
        ) -> tuple[DependencyEdge, ...] | None:
            """
            Find an exact-support closed trail below one chronological state.

            @ai-generated
            """
            if depth == max_depth:
                return None

            for edge in self._edge_ids_after(current, min_t if min_t is not None else first_time):
                candidate = self._sorted_edges[edge]
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
        """
        Return whether an exact-support temporal closed trail of `order` agents exists.

        @ai-generated
        """
        return bool(self.closed_trail_of_order(order))

    def closed_trail_orders(self) -> frozenset[int]:
        """
        Return every exact support order realized by a temporal closed trail.

        @ai-generated
        """
        return frozenset(order for order in range(2, len(self.vertices) + 1) if self.has_closed_trail_of_order(order))

    def interdependence_order(self) -> int:
        """
        Return the largest exact closed-trail support order, or `0` when absent.

        @ai-generated
        """
        return max(self.closed_trail_orders(), default=0)

    def longest_closed_trail(self) -> list[DependencyEdge]:
        """
        Return a longest non-decreasing temporal closed trail.

        Unlike exact-order recognition, this diagnostic method is bounded by
        the finite temporal-edge set rather than the irreducible witness bound,
        so it preserves redundant edges in a concrete trajectory.

        @ai-generated
        """
        best: tuple[DependencyEdge, ...] = ()
        if not self._sorted_edges:
            return []
        first_time = self._sorted_edges[0].t

        def dfs(
            anchor: AgentId,
            current: AgentId,
            min_t: int | None,
            current_time_arcs: frozenset[tuple[AgentId, AgentId]],
            path: tuple[DependencyEdge, ...],
        ) -> None:
            """
            Explore all finite temporal closed trails rooted at one agent.

            @ai-generated
            """
            nonlocal best
            if len(path) == len(self._sorted_edges):
                return

            for edge_id in self._edge_ids_after(current, min_t if min_t is not None else first_time):
                candidate = self._sorted_edges[edge_id]
                if candidate.helper == candidate.beneficiary:
                    continue
                static_arc = (candidate.helper, candidate.beneficiary)
                if candidate.t == min_t:
                    if static_arc in current_time_arcs:
                        continue
                    next_time_arcs = current_time_arcs | {static_arc}
                else:
                    next_time_arcs = frozenset({static_arc})

                next_path = (*path, candidate)
                if candidate.beneficiary == anchor and len(next_path) > len(best):
                    best = next_path
                dfs(anchor, candidate.beneficiary, candidate.t, next_time_arcs, next_path)

        for anchor in self.vertices:
            dfs(anchor, anchor, None, frozenset(), ())
        return list(best)

    def longest_cycle_order(self):
        """
        Return the largest legacy simple temporal cycle order, or `0` if none exists.

        @ai-generated
        """
        return len(self.longest_cycle())

    def has_cycle(self):
        """
        Return whether a legacy simple help cycle exists.

        @ai-generated
        """
        return self.longest_cycle_order() >= 2

    def profile(self):
        """Summarise the graph into a `TrajectoryProfile`."""
        from .profile import PlanProfile

        return PlanProfile(self)

    @staticmethod
    def empty():
        return TemporalCooperationGraph([])
