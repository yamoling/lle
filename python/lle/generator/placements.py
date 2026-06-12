"""Placement strategies for world layout generation.

Each public function places one category of entity onto a candidate layout,
returning the placed items and the updated set of reserved cells.
"""

from __future__ import annotations

import random
from dataclasses import dataclass
from typing import Literal

from lle.tiles import Direction
from lle.types import Position

from .geometry import beam_tiles, place_wall_shapes, points_out_immediately


class LayoutRetry(Exception):
    """Raised when sampling produced an unusable layout."""


OPPOSITE_EDGE: dict[str, str] = {
    "left": "right",
    "right": "left",
    "top": "bottom",
    "bottom": "top",
}

ALL_DIRS = [Direction.NORTH, Direction.SOUTH, Direction.EAST, Direction.WEST]


@dataclass
class PlacementCtx:
    """Ephemeral per-attempt state shared between placement stages."""

    edge: Literal["left", "right", "top", "bottom"] | None = None
    lane_ids: list[int] | None = None
    agent_anchor: tuple[int, int] | None = None
    exit_anchor: tuple[int, int] | None = None


def cluster_shape(n_agents: int) -> tuple[int, int]:
    match n_agents:
        case 1:
            return (1, 1)
        case 2:
            return random.choice([(1, 2), (2, 1)])
        case 3:
            return random.choice([(1, 3), (3, 1)])
        case 4:
            return random.choice([(2, 2), (1, 4), (4, 1)])
        case _:
            raise NotImplementedError()


# ---------------------------------------------------------------------------
# Agent placement
# ---------------------------------------------------------------------------


def place_agents(
    mode: Literal["random", "edge", "clustered"],
    n_agents: int,
    height: int,
    width: int,
    rng: random.Random,
    ctx: PlacementCtx,
) -> tuple[list[Position], set[Position]]:
    if mode == "random":
        all_pos = [(r, c) for r in range(height) for c in range(width)]
        agents = rng.sample(all_pos, n_agents)

    elif mode == "edge":
        edge = rng.choice(("left", "right", "top", "bottom"))
        ctx.edge = edge
        if edge in ("left", "right"):
            col = 0 if edge == "left" else width - 1
            if height < n_agents:
                raise LayoutRetry()
            lane_ids = sorted(rng.sample(range(height), n_agents))
            ctx.lane_ids = lane_ids
            agents = [(r, col) for r in lane_ids]
        else:
            row = 0 if edge == "top" else height - 1
            if width < n_agents:
                raise LayoutRetry()
            lane_ids = sorted(rng.sample(range(width), n_agents))
            ctx.lane_ids = lane_ids
            agents = [(row, c) for c in lane_ids]

    elif mode == "clustered":
        cluster_h, cluster_w = cluster_shape(n_agents)
        if cluster_h > height or cluster_w > width:
            raise LayoutRetry()
        anchor_r = rng.randint(0, height - cluster_h)
        anchor_c = rng.randint(0, width - cluster_w)
        ctx.agent_anchor = (anchor_r, anchor_c)
        cells = [(anchor_r + dr, anchor_c + dc) for dr in range(cluster_h) for dc in range(cluster_w)]
        agents = cells[:n_agents]

    else:
        raise ValueError(f"Unknown starts mode: {mode!r}")

    reserved: set[Position] = set(agents)
    return agents, reserved  # type: ignore[return-value]


# ---------------------------------------------------------------------------
# Exit placement
# ---------------------------------------------------------------------------


def place_exits(
    mode: Literal["random", "edge", "cluster", "opposite"],
    n_agents: int,
    height: int,
    width: int,
    rng: random.Random,
    reserved: set[Position],
    ctx: PlacementCtx,
) -> tuple[list[Position], set[Position]]:
    if mode == "random":
        free = [(r, c) for r in range(height) for c in range(width) if (r, c) not in reserved]
        if len(free) < n_agents:
            raise LayoutRetry()
        exits = rng.sample(free, n_agents)

    elif mode == "edge":
        edge = rng.choice(["left", "right", "top", "bottom"])
        exits = _exits_on_edge(edge, n_agents, height, width, rng, reserved)

    elif mode == "cluster":
        exits = _exits_as_cluster(n_agents, height, width, rng, reserved, ctx)

    elif mode == "opposite":
        if ctx.edge is not None:
            opp = OPPOSITE_EDGE[ctx.edge]
            exits = _exits_on_edge(opp, n_agents, height, width, rng, reserved, lane_ids=ctx.lane_ids)
        elif ctx.agent_anchor is not None:
            exits = _exits_as_opposite_cluster(n_agents, height, width, reserved, ctx)
        else:
            raise LayoutRetry()

    else:
        raise ValueError(f"Unknown exits mode: {mode!r}")

    if any(e in reserved for e in exits):
        raise LayoutRetry()
    return exits, reserved | set(exits)  # type: ignore[return-value]


def _exits_on_edge(
    edge: str,
    n_agents: int,
    height: int,
    width: int,
    rng: random.Random,
    reserved: set[Position],
    lane_ids: list[int] | None = None,
) -> list[Position]:
    if edge in ("left", "right"):
        col = 0 if edge == "left" else width - 1
        if lane_ids is not None:
            exits = [(r, col) for r in lane_ids]
        else:
            if height < n_agents:
                raise LayoutRetry()
            ids = sorted(rng.sample(range(height), n_agents))
            exits = [(r, col) for r in ids]
    else:
        row = 0 if edge == "top" else height - 1
        if lane_ids is not None:
            exits = [(row, c) for c in lane_ids]
        else:
            if width < n_agents:
                raise LayoutRetry()
            ids = sorted(rng.sample(range(width), n_agents))
            exits = [(row, c) for c in ids]
    return exits  # type: ignore[return-value]


def _exits_as_cluster(
    n_agents: int,
    height: int,
    width: int,
    rng: random.Random,
    reserved: set[Position],
    ctx: PlacementCtx,
) -> list[Position]:
    cluster_h, cluster_w = cluster_shape(n_agents)
    if cluster_h > height or cluster_w > width:
        raise LayoutRetry()
    for _ in range(64):
        anchor_r = rng.randint(0, height - cluster_h)
        anchor_c = rng.randint(0, width - cluster_w)
        cells = [(anchor_r + dr, anchor_c + dc) for dr in range(cluster_h) for dc in range(cluster_w)]
        exit_cells = cells[:n_agents]
        if not any(c in reserved for c in exit_cells):
            ctx.exit_anchor = (anchor_r, anchor_c)
            return exit_cells  # type: ignore[return-value]
    raise LayoutRetry()


def _exits_as_opposite_cluster(
    n_agents: int,
    height: int,
    width: int,
    reserved: set[Position],
    ctx: PlacementCtx,
) -> list[Position]:
    cluster_h, cluster_w = cluster_shape(n_agents)
    agent_r, agent_c = ctx.agent_anchor  # type: ignore[misc]
    anchor_r = max(0, min(height - cluster_h - agent_r, height - cluster_h))
    anchor_c = max(0, min(width - cluster_w - agent_c, width - cluster_w))
    ctx.exit_anchor = (anchor_r, anchor_c)
    cells = [(anchor_r + dr, anchor_c + dc) for dr in range(cluster_h) for dc in range(cluster_w)]
    exits = cells[:n_agents]
    if any(e in reserved for e in exits):
        raise LayoutRetry()
    return exits  # type: ignore[return-value]


# ---------------------------------------------------------------------------
# Laser placement
# ---------------------------------------------------------------------------


def _beam_satisfies_span(
    tiles: list[Position],
    src: Position,
    direction: Direction,
    span: int | Literal["any", "across"],
    height: int,
    width: int,
) -> bool:
    if span == "any":
        return len(tiles) >= 2
    if span == "across":
        full = beam_tiles(src, direction, set(), set(), height, width)
        return len(tiles) == len(full)
    return len(tiles) >= span


def _select_lasers(
    candidates: list[tuple[Position, Direction, list[Position]]],
    n_lasers: int,
    rng: random.Random,
    reserved: set[Position],
    reserve_beam: bool,
) -> tuple[list[tuple[int, Position, Direction]], set[Position]]:
    """Greedy non-overlapping selection from shuffled laser candidates."""
    rng.shuffle(candidates)
    lasers: list[tuple[int, Position, Direction]] = []
    used_sources: set[Position] = set()
    all_beam_tiles: set[Position] = set()
    new_reserved = set(reserved)

    for pos, direction, tiles in candidates:
        if len(lasers) >= n_lasers:
            break
        if pos in used_sources or pos in all_beam_tiles:
            continue
        if any(src in tiles for src in used_sources):
            continue
        lasers.append((len(lasers), pos, direction))
        used_sources.add(pos)
        all_beam_tiles.update(tiles)
        new_reserved.add(pos)
        if reserve_beam:
            new_reserved.update(tiles)

    if len(lasers) < n_lasers:
        raise LayoutRetry()
    return lasers, new_reserved


def place_lasers(
    n_lasers: int,
    placement: Literal["free", "cross-agent", "cross-cluster"],
    span: int | Literal["any", "across"],
    n_agents: int,
    height: int,
    width: int,
    rng: random.Random,
    reserved: set[Position],
    ctx: PlacementCtx,
) -> tuple[list[tuple[int, Position, Direction]], set[Position]]:
    if n_lasers == 0:
        return [], reserved
    if placement == "free":
        return _place_lasers_free(n_lasers, span, height, width, rng, reserved)
    if placement == "cross-agent":
        return _place_lasers_cross_agent(n_lasers, span, height, width, rng, reserved, ctx)
    if placement == "cross-cluster":
        return _place_lasers_cross_cluster(n_lasers, span, n_agents, height, width, rng, reserved, ctx)
    raise ValueError(f"Unknown laser_placement: {placement!r}")


def _place_lasers_free(
    n_lasers: int,
    span: int | Literal["any", "across"],
    height: int,
    width: int,
    rng: random.Random,
    reserved: set[Position],
) -> tuple[list[tuple[int, Position, Direction]], set[Position]]:
    candidates: list[tuple[Position, Direction, list[Position]]] = []
    for r in range(height):
        for c in range(width):
            pos = (r, c)
            if pos in reserved:
                continue
            for direction in ALL_DIRS:
                if points_out_immediately(pos, direction, height, width):
                    continue
                tiles = beam_tiles(pos, direction, set(), set(), height, width)
                if len(tiles) < 2:
                    continue
                if any(t in reserved for t in tiles):
                    continue
                if not _beam_satisfies_span(tiles, pos, direction, span, height, width):
                    continue
                candidates.append((pos, direction, tiles))
    return _select_lasers(candidates, n_lasers, rng, reserved, reserve_beam=False)


def _place_lasers_cross_agent(
    n_lasers: int,
    span: int | Literal["any", "across"],
    height: int,
    width: int,
    rng: random.Random,
    reserved: set[Position],
    ctx: PlacementCtx,
) -> tuple[list[tuple[int, Position, Direction]], set[Position]]:
    """Structural lasers perpendicular to agent lanes, each crossing all of them."""
    edge = ctx.edge
    lane_ids = ctx.lane_ids
    assert edge is not None and lane_ids is not None

    lane_set = set(lane_ids)
    min_lane, max_lane = min(lane_ids), max(lane_ids)
    candidates: list[tuple[Position, Direction, list[Position]]] = []

    if edge in ("left", "right"):
        before_band = [r for r in range(height) if r not in lane_set and r < min_lane]
        after_band = [r for r in range(height) if r not in lane_set and r > max_lane]
        for row in before_band:
            for col in range(width):
                pos = (row, col)
                if pos in reserved:
                    continue
                tiles = beam_tiles(pos, Direction.SOUTH, set(), set(), height, width)
                if not _beam_satisfies_span(tiles, pos, Direction.SOUTH, span, height, width):
                    continue
                if not lane_set.issubset(t[0] for t in tiles):
                    continue
                if any(t in reserved for t in tiles):
                    continue
                candidates.append((pos, Direction.SOUTH, tiles))
        for row in after_band:
            for col in range(width):
                pos = (row, col)
                if pos in reserved:
                    continue
                tiles = beam_tiles(pos, Direction.NORTH, set(), set(), height, width)
                if not _beam_satisfies_span(tiles, pos, Direction.NORTH, span, height, width):
                    continue
                if not lane_set.issubset(t[0] for t in tiles):
                    continue
                if any(t in reserved for t in tiles):
                    continue
                candidates.append((pos, Direction.NORTH, tiles))
    else:
        before_band = [c for c in range(width) if c not in lane_set and c < min_lane]
        after_band = [c for c in range(width) if c not in lane_set and c > max_lane]
        for col in before_band:
            for row in range(height):
                pos = (row, col)
                if pos in reserved:
                    continue
                tiles = beam_tiles(pos, Direction.EAST, set(), set(), height, width)
                if not _beam_satisfies_span(tiles, pos, Direction.EAST, span, height, width):
                    continue
                if not lane_set.issubset(t[1] for t in tiles):
                    continue
                if any(t in reserved for t in tiles):
                    continue
                candidates.append((pos, Direction.EAST, tiles))
        for col in after_band:
            for row in range(height):
                pos = (row, col)
                if pos in reserved:
                    continue
                tiles = beam_tiles(pos, Direction.WEST, set(), set(), height, width)
                if not _beam_satisfies_span(tiles, pos, Direction.WEST, span, height, width):
                    continue
                if not lane_set.issubset(t[1] for t in tiles):
                    continue
                if any(t in reserved for t in tiles):
                    continue
                candidates.append((pos, Direction.WEST, tiles))

    if not candidates:
        raise LayoutRetry()
    return _select_lasers(candidates, n_lasers, rng, reserved, reserve_beam=True)


def _place_lasers_cross_cluster(
    n_lasers: int,
    span: int | Literal["any", "across"],
    n_agents: int,
    height: int,
    width: int,
    rng: random.Random,
    reserved: set[Position],
    ctx: PlacementCtx,
) -> tuple[list[tuple[int, Position, Direction]], set[Position]]:
    """Corridor lasers alternating from opposite sides between the two clusters."""
    cluster_h, cluster_w = cluster_shape(n_agents)
    agent_anchor = ctx.agent_anchor
    exit_anchor = ctx.exit_anchor
    if agent_anchor is None or exit_anchor is None:
        raise LayoutRetry()

    agent_bottom = agent_anchor[0] + cluster_h - 1
    agent_right = agent_anchor[1] + cluster_w - 1
    exit_top = exit_anchor[0]
    exit_left = exit_anchor[1]

    row_gap = exit_top - agent_bottom - 1
    col_gap = exit_left - agent_right - 1

    if row_gap >= n_lasers:
        corridor_rows = list(range(agent_bottom + 1, exit_top))
        rng.shuffle(corridor_rows)
        chosen = sorted(corridor_rows[:n_lasers])
        return _corridor_lasers_horizontal(chosen, span, height, width, reserved)
    if col_gap >= n_lasers:
        corridor_cols = list(range(agent_right + 1, exit_left))
        rng.shuffle(corridor_cols)
        chosen = sorted(corridor_cols[:n_lasers])
        return _corridor_lasers_vertical(chosen, span, height, width, reserved)
    raise LayoutRetry()


def _corridor_lasers_horizontal(
    corridor_rows: list[int],
    span: int | Literal["any", "across"],
    height: int,
    width: int,
    reserved: set[Position],
) -> tuple[list[tuple[int, Position, Direction]], set[Position]]:
    """E/W lasers placed on corridor rows from alternating sides."""
    lasers: list[tuple[int, Position, Direction]] = []
    new_reserved = set(reserved)
    for i, row in enumerate(corridor_rows):
        pos: Position
        if i % 2 == 0:
            pos = (row, 0)
            direction = Direction.EAST
        else:
            pos = (row, width - 1)
            direction = Direction.WEST
        if pos in new_reserved:
            raise LayoutRetry()
        tiles = beam_tiles(pos, direction, set(), set(), height, width)
        if not _beam_satisfies_span(tiles, pos, direction, span, height, width):
            raise LayoutRetry()
        lasers.append((i, pos, direction))
        new_reserved.add(pos)
        new_reserved.update(tiles)
    return lasers, new_reserved


def _corridor_lasers_vertical(
    corridor_cols: list[int],
    span: int | Literal["any", "across"],
    height: int,
    width: int,
    reserved: set[Position],
) -> tuple[list[tuple[int, Position, Direction]], set[Position]]:
    """N/S lasers placed on corridor cols from alternating sides."""
    lasers: list[tuple[int, Position, Direction]] = []
    new_reserved = set(reserved)
    for i, col in enumerate(corridor_cols):
        pos: Position
        if i % 2 == 0:
            pos = (0, col)
            direction = Direction.SOUTH
        else:
            pos = (height - 1, col)
            direction = Direction.NORTH
        if pos in new_reserved:
            raise LayoutRetry()
        tiles = beam_tiles(pos, direction, set(), set(), height, width)
        if not _beam_satisfies_span(tiles, pos, direction, span, height, width):
            raise LayoutRetry()
        lasers.append((i, pos, direction))
        new_reserved.add(pos)
        new_reserved.update(tiles)
    return lasers, new_reserved


# ---------------------------------------------------------------------------
# Wall placement
# ---------------------------------------------------------------------------


def place_walls(
    n_walls: int,
    style: Literal["individual", "shapes"],
    reserved: set[Position],
    height: int,
    width: int,
    rng: random.Random,
) -> list[Position]:
    free = [(r, c) for r in range(height) for c in range(width) if (r, c) not in reserved]
    if style == "shapes":
        return place_wall_shapes(free, n_walls, rng)
    n = min(n_walls, len(free))
    return rng.sample(free, n)  # type: ignore[return-value]
