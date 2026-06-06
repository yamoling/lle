"""Classify cooperation structure from SAT models.

The analyzer reuses the standard and strict solvers, then reconstructs helper
relationships from the SAT model and the beam geometry.
"""

from __future__ import annotations

from collections import defaultdict
from dataclasses import dataclass
from typing import Sequence, overload

from ..world import Action, World
from ._internal.grid import is_within_bounds
from ._internal.types import Position, laser_sources_from_world
from .cooperation_level import CooperationLevel
from .incremental_solver import solve, solve_no_cooperation


@dataclass(frozen=True)
class _HelperEvent:
    helper: int
    beneficiary: int
    time: int


def _classify_in_interval(world: World, t_min: int, t_max: int) -> CooperationLevel | None:
    """Return the cooperation level for ``world`` restricted to the interval ``[t_min, t_max]``.

    Unlike :func:`classify`, both the cooperative-plan search and the non-cooperative check
    are bounded below by ``t_min``.  This is the correct function to use when generating
    levels with a guaranteed minimum solution length: a non-cooperative plan shorter than
    ``t_min`` steps is irrelevant because agents cannot reach it.

    Returns ``None`` if ``world`` is not solvable in ``[t_min, t_max]`` steps.
    """
    standard_plan = solve(world, t_min=t_min, t_max=t_max)
    if standard_plan is None:
        return None
    positions_by_time = _positions_by_time_from_trajectory(world, standard_plan)
    helper_events = _extract_helper_events(world, positions_by_time)
    dependency_edges = {(e.helper, e.beneficiary) for e in helper_events}
    if not dependency_edges:
        return CooperationLevel.INDEPENDENT
    no_coop_plan = solve_no_cooperation(world, t_min=t_min, t_max=t_max)
    if no_coop_plan is not None:
        return CooperationLevel.INDEPENDENT
    return classify_profile(
        dependency_edges=dependency_edges,
        mutual_pairs=_mutual_pairs(dependency_edges),
        largest_scc_size=_largest_scc_size(dependency_edges, world.n_agents),
        longest_chain_length=_longest_chain_length(dependency_edges, world.n_agents),
        num_agents=world.n_agents,
    )


@overload
def classify(world: World, t_min: int, t_max: int, /) -> CooperationLevel | None: ...


@overload
def classify(world: World, t_max: int, /) -> CooperationLevel | None: ...


@overload
def classify(world: World, trajectory: Sequence[tuple[Action, ...]], /) -> CooperationLevel | None: ...


def classify(world: World, *args):
    match args:
        case (int(t_max),):
            return _classify_in_interval(world, 0, t_max)
        case (int(t_min), int(t_max)):
            return _classify_in_interval(world, t_min, t_max)
        case (trajectory,):
            return _classify_trajectory(world, trajectory)
        case other:
            raise ValueError(f"Invalid arguments: {other}")


def _classify_from_scratch(world: World, t_max: int) -> CooperationLevel | None:
    """Return the precise CooperationLevel for ``world`` within ``t_max`` steps, or None if the level is not solvable."""
    # 1) Check that the world is solvable within t_max steps
    standard_plan = solve(world, t_max=t_max)
    if standard_plan is None:
        return None
    # 2) Check if the proposed plan is already independent (no agent shields a beam for another).
    positions_by_time = _positions_by_time_from_trajectory(world, standard_plan)
    helper_events = _extract_helper_events(world, positions_by_time)
    dependency_edges = {(e.helper, e.beneficiary) for e in helper_events}
    if len(dependency_edges) == 0:
        return CooperationLevel.INDEPENDENT
    # 3) Check that no non-blocking path exists within t_max steps.
    # Uses no_blocking_clauses (CooperationConstraints) instead of StrictLaserConstraints.
    no_coop_plan = solve_no_cooperation(world, t_max=t_max)
    cooperation_required = no_coop_plan is None
    if not cooperation_required:
        return CooperationLevel.INDEPENDENT
    # 4) Finally, profile the initial proposed solution
    num_agents = world.n_agents
    return classify_profile(
        dependency_edges=dependency_edges,
        mutual_pairs=_mutual_pairs(dependency_edges),
        largest_scc_size=_largest_scc_size(dependency_edges, num_agents),
        longest_chain_length=_longest_chain_length(dependency_edges, num_agents),
        num_agents=num_agents,
    )


def _classify_trajectory(world: World, trajectory: Sequence[Action | Sequence[Action]]) -> CooperationLevel:
    """Return the cooperation profile induced by a concrete action trajectory.

    Unlike :func:`classify`, this does not search for an alternative SAT plan.
    It only analyses the provided trajectory and therefore can characterize any
    executable sequence of joint actions for the given world.
    """
    world.reset()
    positions_by_time = _positions_by_time_from_trajectory(world, trajectory)
    helper_events = _extract_helper_events(world, positions_by_time)
    dependency_edges = {(e.helper, e.beneficiary) for e in helper_events}
    if len(dependency_edges) == 0:
        return CooperationLevel.INDEPENDENT
    return classify_profile(
        dependency_edges=dependency_edges,
        mutual_pairs=_mutual_pairs(dependency_edges),
        largest_scc_size=_largest_scc_size(dependency_edges, world.n_agents),
        longest_chain_length=_longest_chain_length(dependency_edges, world.n_agents),
        num_agents=world.n_agents,
    )


# ---------------------------------------------------------------------------
# SAT model -> agent positions per timestep
# ---------------------------------------------------------------------------


def _positions_by_time_from_trajectory(world: World, trajectory: Sequence[Action | Sequence[Action]]):
    world.reset()
    positions = [world.agents_positions]
    for joint_action in trajectory:
        world.step(joint_action)
        positions.append(world.agents_positions)
    return positions


# ---------------------------------------------------------------------------
# Helper-event extraction (own-colour beam shielding)
# ---------------------------------------------------------------------------


def _extract_helper_events(world: World, positions_by_time: list[list[Position]]) -> set[_HelperEvent]:
    events: set[_HelperEvent] = set()
    beam_paths = _raw_beam_paths(world)

    for t, positions in enumerate(positions_by_time):
        for helper, helper_pos in enumerate(positions):
            for _src_pos, path in beam_paths.get(helper, []):
                if helper_pos not in path:
                    continue
                helper_index = path.index(helper_pos)
                downstream = set(path[helper_index + 1 :])
                if not downstream:
                    continue
                for beneficiary, beneficiary_pos in enumerate(positions):
                    if beneficiary == helper:
                        continue
                    if beneficiary_pos in downstream:
                        events.add(_HelperEvent(helper=helper, beneficiary=beneficiary, time=t))
    return events


def _raw_beam_paths(world: World) -> dict[int, list[tuple[tuple[int, int], list[tuple[int, int]]]]]:
    paths: dict[int, list[tuple[tuple[int, int], list[tuple[int, int]]]]] = defaultdict(list)
    wall_positions = frozenset(world.wall_pos)
    sources = laser_sources_from_world(world)
    source_positions = {src.position for src in sources}

    for laser in sources:
        di, dj = laser.direction
        x, y = laser.position
        x += di
        y += dj
        path: list[tuple[int, int]] = []
        while is_within_bounds(world, (x, y)):
            if (x, y) in wall_positions or (x, y) in source_positions:
                break
            path.append((x, y))
            x += di
            y += dj
        paths[laser.color].append((laser.position, path))
    return paths


# ---------------------------------------------------------------------------
# Pure graph metrics over the dependency-edge set
# ---------------------------------------------------------------------------


def _mutual_pairs(edges: set[tuple[int, int]]) -> set[tuple[int, int]]:
    result: set[tuple[int, int]] = set()
    for src, dst in edges:
        if (dst, src) in edges and src < dst:
            result.add((src, dst))
    return result


def _largest_scc_size(edges: set[tuple[int, int]], num_agents: int) -> int:
    if num_agents == 0:
        return 0
    adjacency: dict[int, set[int]] = {i: set() for i in range(num_agents)}
    reverse: dict[int, set[int]] = {i: set() for i in range(num_agents)}
    for src, dst in edges:
        adjacency[src].add(dst)
        reverse[dst].add(src)

    visited: set[int] = set()
    order: list[int] = []

    def dfs(node: int) -> None:
        stack = [(node, iter(adjacency[node]))]
        visited.add(node)
        while stack:
            current, it = stack[-1]
            for nxt in it:
                if nxt not in visited:
                    visited.add(nxt)
                    stack.append((nxt, iter(adjacency[nxt])))
                    break
            else:
                order.append(current)
                stack.pop()

    for node in range(num_agents):
        if node not in visited:
            dfs(node)

    visited.clear()
    largest = 1

    def reverse_dfs(start: int) -> int:
        size = 0
        stack = [start]
        visited.add(start)
        while stack:
            node = stack.pop()
            size += 1
            for nxt in reverse[node]:
                if nxt not in visited:
                    visited.add(nxt)
                    stack.append(nxt)
        return size

    for node in reversed(order):
        if node in visited:
            continue
        largest = max(largest, reverse_dfs(node))
    return largest


def _longest_chain_length(edges: set[tuple[int, int]], num_agents: int) -> int:
    adjacency: dict[int, set[int]] = {i: set() for i in range(num_agents)}
    indegree: dict[int, int] = {i: 0 for i in range(num_agents)}
    for src, dst in edges:
        if dst not in adjacency[src]:
            adjacency[src].add(dst)
            indegree[dst] += 1

    queue = [node for node in range(num_agents) if indegree[node] == 0]
    topo: list[int] = []
    while queue:
        node = queue.pop()
        topo.append(node)
        for nxt in adjacency[node]:
            indegree[nxt] -= 1
            if indegree[nxt] == 0:
                queue.append(nxt)

    if len(topo) != num_agents:
        return 0

    dist: dict[int, int] = {i: 0 for i in range(num_agents)}
    for node in topo:
        for nxt in adjacency[node]:
            dist[nxt] = max(dist[nxt], dist[node] + 1)
    return max(dist.values(), default=0)


# ---------------------------------------------------------------------------
# Classification rule (cooperative branch)
# ---------------------------------------------------------------------------


def classify_profile(
    *,
    dependency_edges: set[tuple[int, int]],
    mutual_pairs: set[tuple[int, int]],
    largest_scc_size: int,
    longest_chain_length: int,
    num_agents: int,
) -> CooperationLevel:
    if largest_scc_size == num_agents and num_agents > 1:
        return CooperationLevel.FULLY_COUPLED
    if mutual_pairs:
        return CooperationLevel.MUTUAL
    indegree: dict[int, int] = defaultdict(int)
    outdegree: dict[int, int] = defaultdict(int)
    nodes: set[int] = set()
    for src, dst in dependency_edges:
        indegree[dst] += 1
        outdegree[src] += 1
        nodes.add(src)
        nodes.add(dst)
    if any(count >= 2 for count in indegree.values()):
        return CooperationLevel.DISTRIBUTED
    if (
        dependency_edges
        and longest_chain_length >= 2
        and all(indegree[n] <= 1 for n in nodes)
        and all(outdegree[n] <= 1 for n in nodes)
        and longest_chain_length >= max(1, len(nodes) - 1)
    ):
        return CooperationLevel.CHAIN
    if dependency_edges:
        return CooperationLevel.ASYMMETRIC
    return CooperationLevel.COOPERATIVE
