"""Customizable layout generator with explicit control over every placement decision."""

from __future__ import annotations

import random
from dataclasses import dataclass
from typing import Literal

from lle.tiles import Direction
from lle.types import Position

from ._candidates import CandidateLayout
from ._geometry import beam_tiles, points_out_immediately
from ._shapes import place_wall_shapes
from .generator import Generator, _LayoutRetry
from .world_filter import WorldFilter

# ---------------------------------------------------------------------------
# Internal config types
# ---------------------------------------------------------------------------

_OPPOSITE_EDGE: dict[str, str] = {
    "left": "right",
    "right": "left",
    "top": "bottom",
    "bottom": "top",
}

_ALL_DIRS = [Direction.NORTH, Direction.SOUTH, Direction.EAST, Direction.WEST]


def _cluster_shape(n_agents: int) -> tuple[int, int]:
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


@dataclass
class _AgentConfig:
    mode: Literal["random", "edge", "clustered"]


@dataclass
class _ExitConfig:
    mode: Literal["random", "edge", "cluster", "opposite"]


@dataclass
class _LaserConfig:
    n: int
    placement: Literal["free", "cross-agent", "cross-cluster"]
    span: int | Literal["any", "across"]


@dataclass
class _WallConfig:
    n: int
    style: Literal["individual", "shapes"]


@dataclass
class _PlacementCtx:
    """Ephemeral per-attempt state passed from _place_agents to _place_exits/_place_lasers."""

    edge: Literal["left", "right", "top", "bottom"] | None = None
    lane_ids: list[int] | None = None
    agent_anchor: tuple[int, int] | None = None
    exit_anchor: tuple[int, int] | None = None


# ---------------------------------------------------------------------------
# CustomGenerator
# ---------------------------------------------------------------------------


class CustomGenerator(Generator):
    """Generator with explicit, composable placement strategies.

    Parameters
    ----------
    starts:
        Agent start placement — ``"random"`` (anywhere), ``"edge"`` (one edge,
        random direction each attempt), or ``"clustered"`` (rectangular group,
        random anchor each attempt).
    exits:
        Exit placement — same modes as ``starts``, plus ``"opposite"`` which
        mirrors the agent edge/cluster to the far side of the grid.
    n_lasers:
        Number of laser sources (0 = no lasers).
    laser_placement:
        ``"free"`` — valid position anywhere outside reserved cells.
        ``"cross-agent"`` — structural laser perpendicular to agent lanes,
        crossing all of them (requires ``starts="edge"``).
        ``"cross-cluster"`` — corridor laser between start and exit clusters
        (requires ``starts="clustered"`` and ``exits`` in
        ``{"opposite", "cluster"}``).
    laser_span:
        Minimum beam length. ``"any"`` enforces the 2-tile minimum. ``"across"``
        requires the beam to reach the far grid boundary untruncated. An integer
        sets an explicit minimum tile count (>= 2).
    n_walls:
        Number of wall tiles. ``"auto"`` uses ~10 % of the grid.
    walls_style:
        ``"individual"`` places single-cell walls; ``"shapes"`` groups them into
        connected bars / L-shapes / 2×2 blocks.
    filter:
        A :class:`WorldFilter` applied after layout generation. ``None`` accepts
        any geometrically valid world.
    """

    def __init__(
        self,
        *,
        width: int,
        height: int,
        n_agents: int = 2,
        starts: Literal["random", "edge", "clustered"] = "random",
        exits: Literal["random", "edge", "cluster", "opposite"] = "random",
        n_lasers: int = 0,
        laser_placement: Literal["free", "cross-agent", "cross-cluster"] = "free",
        laser_span: int | Literal["any", "across"] = "any",
        n_walls: int | Literal["auto"] = "auto",
        walls_style: Literal["individual", "shapes"] = "individual",
        filter: WorldFilter | None = None,
    ):
        # Cross-parameter validation
        if exits == "opposite" and starts == "random":
            raise ValueError("exits='opposite' requires starts='edge' or starts='clustered', not 'random'.")
        if laser_placement == "cross-agent" and starts != "edge":
            raise ValueError("laser_placement='cross-agent' requires starts='edge'.")
        if laser_placement == "cross-cluster" and starts != "clustered":
            raise ValueError("laser_placement='cross-cluster' requires starts='clustered'.")
        if laser_placement == "cross-cluster" and exits not in ("opposite", "cluster"):
            raise ValueError("laser_placement='cross-cluster' requires exits='opposite' or exits='cluster'.")
        if isinstance(laser_span, int) and laser_span < 2:
            raise ValueError(f"laser_span must be >= 2, got {laser_span}.")

        resolved_n_walls = (width * height) // 10 if n_walls == "auto" else n_walls

        super().__init__(
            width=width,
            height=height,
            n_agents=n_agents,
            n_lasers=n_lasers,
            n_walls=resolved_n_walls,
            world_filter=filter,
        )

        self._agent_cfg = _AgentConfig(mode=starts)
        self._exit_cfg = _ExitConfig(mode=exits)
        self._laser_cfg = _LaserConfig(n=n_lasers, placement=laser_placement, span=laser_span)
        self._wall_cfg = _WallConfig(n=resolved_n_walls, style=walls_style)

    # ------------------------------------------------------------------
    # Generator protocol
    # ------------------------------------------------------------------

    def _make_candidate_layout(self) -> CandidateLayout:
        ctx = _PlacementCtx()
        agents, reserved = self._place_agents(ctx)
        exits, reserved = self._place_exits(reserved, ctx)
        lasers, reserved = self._place_lasers(reserved, ctx)
        walls = self._place_walls(reserved)
        layout = CandidateLayout(self.height, self.width, agents=agents, exits=exits, walls=walls, lasers=lasers)
        if not layout.is_geometry_valid():
            raise _LayoutRetry()
        return layout

    # ------------------------------------------------------------------
    # Agent placement
    # ------------------------------------------------------------------

    def _place_agents(self, ctx: _PlacementCtx) -> tuple[list[tuple[int, int]], set[tuple[int, int]]]:
        mode = self._agent_cfg.mode

        if mode == "random":
            all_pos = [(r, c) for r in range(self.height) for c in range(self.width)]
            agents = self._rng.sample(all_pos, self.agents)

        elif mode == "edge":
            edge = self._rng.choice(("left", "right", "top", "bottom"))
            ctx.edge = edge
            if edge in ("left", "right"):
                col = 0 if edge == "left" else self.width - 1
                if self.height < self.agents:
                    raise _LayoutRetry()
                lane_ids = sorted(self._rng.sample(range(self.height), self.agents))
                ctx.lane_ids = lane_ids
                agents = [(r, col) for r in lane_ids]
            else:  # top / bottom
                row = 0 if edge == "top" else self.height - 1
                if self.width < self.agents:
                    raise _LayoutRetry()
                lane_ids = sorted(self._rng.sample(range(self.width), self.agents))
                ctx.lane_ids = lane_ids
                agents = [(row, c) for c in lane_ids]

        elif mode == "clustered":
            cluster_h, cluster_w = _cluster_shape(self.agents)
            if cluster_h > self.height or cluster_w > self.width:
                raise _LayoutRetry()
            anchor_r = self._rng.randint(0, self.height - cluster_h)
            anchor_c = self._rng.randint(0, self.width - cluster_w)
            ctx.agent_anchor = (anchor_r, anchor_c)
            cells = [(anchor_r + dr, anchor_c + dc) for dr in range(cluster_h) for dc in range(cluster_w)]
            agents = cells[: self.agents]

        else:
            raise ValueError(f"Unknown starts mode: {mode!r}")

        reserved: set[tuple[int, int]] = set(agents)
        return agents, reserved

    # ------------------------------------------------------------------
    # Exit placement
    # ------------------------------------------------------------------

    def _place_exits(
        self,
        reserved: set[tuple[int, int]],
        ctx: _PlacementCtx,
    ) -> tuple[list[tuple[int, int]], set[tuple[int, int]]]:
        mode = self._exit_cfg.mode

        if mode == "random":
            free = [(r, c) for r in range(self.height) for c in range(self.width) if (r, c) not in reserved]
            if len(free) < self.agents:
                raise _LayoutRetry()
            exits = self._rng.sample(free, self.agents)

        elif mode == "edge":
            edge = self._rng.choice(["left", "right", "top", "bottom"])
            exits = self._exits_on_edge(edge, reserved)

        elif mode == "cluster":
            exits = self._exits_as_cluster(reserved, ctx)

        elif mode == "opposite":
            if ctx.edge is not None:
                opp = _OPPOSITE_EDGE[ctx.edge]
                exits = self._exits_on_edge(opp, reserved, lane_ids=ctx.lane_ids)
            elif ctx.agent_anchor is not None:
                exits = self._exits_as_opposite_cluster(reserved, ctx)
            else:
                raise _LayoutRetry()

        else:
            raise ValueError(f"Unknown exits mode: {mode!r}")

        if any(e in reserved for e in exits):
            raise _LayoutRetry()

        return exits, reserved | set(exits)

    def _exits_on_edge(
        self,
        edge: str,
        reserved: set[tuple[int, int]],
        lane_ids: list[int] | None = None,
    ) -> list[tuple[int, int]]:
        if edge in ("left", "right"):
            col = 0 if edge == "left" else self.width - 1
            if lane_ids is not None:
                exits = [(r, col) for r in lane_ids]
            else:
                if self.height < self.agents:
                    raise _LayoutRetry()
                ids = sorted(self._rng.sample(range(self.height), self.agents))
                exits = [(r, col) for r in ids]
        else:
            row = 0 if edge == "top" else self.height - 1
            if lane_ids is not None:
                exits = [(row, c) for c in lane_ids]
            else:
                if self.width < self.agents:
                    raise _LayoutRetry()
                ids = sorted(self._rng.sample(range(self.width), self.agents))
                exits = [(row, c) for c in ids]
        return exits

    def _exits_as_cluster(
        self,
        reserved: set[tuple[int, int]],
        ctx: _PlacementCtx,
    ) -> list[tuple[int, int]]:
        cluster_h, cluster_w = _cluster_shape(self.agents)
        if cluster_h > self.height or cluster_w > self.width:
            raise _LayoutRetry()
        for _ in range(64):
            anchor_r = self._rng.randint(0, self.height - cluster_h)
            anchor_c = self._rng.randint(0, self.width - cluster_w)
            cells = [(anchor_r + dr, anchor_c + dc) for dr in range(cluster_h) for dc in range(cluster_w)]
            exit_cells = cells[: self.agents]
            if not any(c in reserved for c in exit_cells):
                ctx.exit_anchor = (anchor_r, anchor_c)
                return exit_cells
        raise _LayoutRetry()

    def _exits_as_opposite_cluster(
        self,
        reserved: set[tuple[int, int]],
        ctx: _PlacementCtx,
    ) -> list[tuple[int, int]]:
        cluster_h, cluster_w = _cluster_shape(self.agents)
        agent_r, agent_c = ctx.agent_anchor  # type: ignore[misc]
        anchor_r = max(0, min(self.height - cluster_h - agent_r, self.height - cluster_h))
        anchor_c = max(0, min(self.width - cluster_w - agent_c, self.width - cluster_w))
        ctx.exit_anchor = (anchor_r, anchor_c)
        cells = [(anchor_r + dr, anchor_c + dc) for dr in range(cluster_h) for dc in range(cluster_w)]
        exits = cells[: self.agents]
        if any(e in reserved for e in exits):
            raise _LayoutRetry()
        return exits

    # ------------------------------------------------------------------
    # Laser placement
    # ------------------------------------------------------------------

    def _beam_satisfies_span(
        self,
        tiles: list[tuple[int, int]],
        src: tuple[int, int],
        direction: Direction,
    ) -> bool:
        span = self._laser_cfg.span
        if span == "any":
            return len(tiles) >= 2
        if span == "across":
            full = beam_tiles(src, direction, set(), set(), self.height, self.width)
            return len(tiles) == len(full)
        return len(tiles) >= span  # int minimum

    def _place_lasers(
        self,
        reserved: set[tuple[int, int]],
        ctx: _PlacementCtx,
    ) -> tuple[list[tuple[int, tuple[int, int], Direction]], set[tuple[int, int]]]:
        if self._laser_cfg.n == 0:
            return [], reserved

        placement = self._laser_cfg.placement
        if placement == "free":
            return self._place_lasers_free(reserved)
        if placement == "cross-agent":
            return self._place_lasers_cross_agent(reserved, ctx)
        if placement == "cross-cluster":
            return self._place_lasers_cross_cluster(reserved, ctx)
        raise ValueError(f"Unknown laser_placement: {placement!r}")

    def _place_lasers_free(
        self,
        reserved: set[tuple[int, int]],
    ) -> tuple[list[tuple[int, tuple[int, int], Direction]], set[tuple[int, int]]]:
        """Greedy free-placement: any valid position outside reserved cells."""
        candidates: list[tuple[tuple[int, int], Direction, list[tuple[int, int]]]] = []
        for r in range(self.height):
            for c in range(self.width):
                pos = (r, c)
                if pos in reserved:
                    continue
                for direction in _ALL_DIRS:
                    if points_out_immediately(pos, direction, self.height, self.width):
                        continue
                    tiles = beam_tiles(pos, direction, set(), set(), self.height, self.width)
                    if len(tiles) < 2:
                        continue
                    if any(t in reserved for t in tiles):
                        continue
                    if not self._beam_satisfies_span(tiles, pos, direction):
                        continue
                    candidates.append((pos, direction, tiles))

        self._rng.shuffle(candidates)
        lasers: list[tuple[int, tuple[int, int], Direction]] = []
        used_sources: set[tuple[int, int]] = set()
        all_beam_tiles: set[tuple[int, int]] = set()
        new_reserved = set(reserved)

        for pos, direction, tiles in candidates:
            if len(lasers) >= self._laser_cfg.n:
                break
            if pos in used_sources:
                continue
            if pos in all_beam_tiles:
                continue
            if any(src in tiles for src in used_sources):
                continue
            lasers.append((len(lasers), pos, direction))
            used_sources.add(pos)
            all_beam_tiles.update(tiles)
            new_reserved.add(pos)

        if len(lasers) < self._laser_cfg.n:
            raise _LayoutRetry()
        return lasers, new_reserved

    def _place_lasers_cross_agent(
        self,
        reserved: set[tuple[int, int]],
        ctx: _PlacementCtx,
    ) -> tuple[list[tuple[int, tuple[int, int], Direction]], set[tuple[int, int]]]:
        """Structural lasers perpendicular to agent lanes, each crossing all of them."""
        edge = ctx.edge
        lane_ids = ctx.lane_ids
        assert edge is not None and lane_ids is not None

        lane_set = set(lane_ids)
        min_lane, max_lane = min(lane_ids), max(lane_ids)
        candidates: list[tuple[tuple[int, int], Direction, list[tuple[int, int]]]] = []

        if edge in ("left", "right"):
            # Lanes are rows; laser fires SOUTH/NORTH through a column
            before_band = [r for r in range(self.height) if r not in lane_set and r < min_lane]
            after_band = [r for r in range(self.height) if r not in lane_set and r > max_lane]
            for row in before_band:
                for col in range(self.width):
                    pos = (row, col)
                    if pos in reserved:
                        continue
                    tiles = beam_tiles(pos, Direction.SOUTH, set(), set(), self.height, self.width)
                    if not self._beam_satisfies_span(tiles, pos, Direction.SOUTH):
                        continue
                    if not lane_set.issubset(t[0] for t in tiles):
                        continue
                    if any(t in reserved for t in tiles):
                        continue
                    candidates.append((pos, Direction.SOUTH, tiles))
            for row in after_band:
                for col in range(self.width):
                    pos = (row, col)
                    if pos in reserved:
                        continue
                    tiles = beam_tiles(pos, Direction.NORTH, set(), set(), self.height, self.width)
                    if not self._beam_satisfies_span(tiles, pos, Direction.NORTH):
                        continue
                    if not lane_set.issubset(t[0] for t in tiles):
                        continue
                    if any(t in reserved for t in tiles):
                        continue
                    candidates.append((pos, Direction.NORTH, tiles))
        else:
            # Lanes are cols; laser fires EAST/WEST through a row
            before_band = [c for c in range(self.width) if c not in lane_set and c < min_lane]
            after_band = [c for c in range(self.width) if c not in lane_set and c > max_lane]
            for col in before_band:
                for row in range(self.height):
                    pos = (row, col)
                    if pos in reserved:
                        continue
                    tiles = beam_tiles(pos, Direction.EAST, set(), set(), self.height, self.width)
                    if not self._beam_satisfies_span(tiles, pos, Direction.EAST):
                        continue
                    if not lane_set.issubset(t[1] for t in tiles):
                        continue
                    if any(t in reserved for t in tiles):
                        continue
                    candidates.append((pos, Direction.EAST, tiles))
            for col in after_band:
                for row in range(self.height):
                    pos = (row, col)
                    if pos in reserved:
                        continue
                    tiles = beam_tiles(pos, Direction.WEST, set(), set(), self.height, self.width)
                    if not self._beam_satisfies_span(tiles, pos, Direction.WEST):
                        continue
                    if not lane_set.issubset(t[1] for t in tiles):
                        continue
                    if any(t in reserved for t in tiles):
                        continue
                    candidates.append((pos, Direction.WEST, tiles))

        if not candidates:
            raise _LayoutRetry()

        self._rng.shuffle(candidates)
        lasers: list[tuple[int, tuple[int, int], Direction]] = []
        used_sources: set[tuple[int, int]] = set()
        all_beam_tiles: set[tuple[int, int]] = set()
        new_reserved = set(reserved)

        for pos, direction, tiles in candidates:
            if len(lasers) >= self._laser_cfg.n:
                break
            if pos in used_sources:
                continue
            if pos in all_beam_tiles:
                continue
            if any(src in tiles for src in used_sources):
                continue
            lasers.append((len(lasers), pos, direction))
            used_sources.add(pos)
            all_beam_tiles.update(tiles)
            new_reserved.add(pos)
            new_reserved.update(tiles)  # reserve beam path so walls can't block it

        if len(lasers) < self._laser_cfg.n:
            raise _LayoutRetry()
        return lasers, new_reserved

    def _place_lasers_cross_cluster(
        self,
        reserved: set[tuple[int, int]],
        ctx: _PlacementCtx,
    ) -> tuple[list[tuple[int, Position, Direction]], set[Position]]:
        """Corridor lasers alternating from opposite sides between the two clusters."""
        cluster_h, cluster_w = _cluster_shape(self.agents)
        agent_anchor = ctx.agent_anchor
        exit_anchor = ctx.exit_anchor
        if agent_anchor is None or exit_anchor is None:
            raise _LayoutRetry()

        agent_bottom = agent_anchor[0] + cluster_h - 1
        agent_right = agent_anchor[1] + cluster_w - 1
        exit_top = exit_anchor[0]
        exit_left = exit_anchor[1]

        row_gap = exit_top - agent_bottom - 1
        col_gap = exit_left - agent_right - 1

        # Try vertical corridor first (between top and bottom clusters), then horizontal
        if row_gap >= self._laser_cfg.n:
            corridor_rows = list(range(agent_bottom + 1, exit_top))
            self._rng.shuffle(corridor_rows)
            chosen = sorted(corridor_rows[: self._laser_cfg.n])
            return self._corridor_lasers_horizontal(chosen, reserved)
        if col_gap >= self._laser_cfg.n:
            corridor_cols = list(range(agent_right + 1, exit_left))
            self._rng.shuffle(corridor_cols)
            chosen = sorted(corridor_cols[: self._laser_cfg.n])
            return self._corridor_lasers_vertical(chosen, reserved)
        raise _LayoutRetry()

    def _corridor_lasers_horizontal(
        self,
        corridor_rows: list[int],
        reserved: set[tuple[int, int]],
    ) -> tuple[list[tuple[int, tuple[int, int], Direction]], set[tuple[int, int]]]:
        """E/W lasers placed on corridor rows from alternating sides."""
        lasers: list[tuple[int, tuple[int, int], Direction]] = []
        new_reserved = set(reserved)
        for i, row in enumerate(corridor_rows):
            if i % 2 == 0:
                pos: tuple[int, int] = (row, 0)
                direction = Direction.EAST
            else:
                pos = (row, self.width - 1)
                direction = Direction.WEST
            if pos in new_reserved:
                raise _LayoutRetry()
            tiles = beam_tiles(pos, direction, set(), set(), self.height, self.width)
            if not self._beam_satisfies_span(tiles, pos, direction):
                raise _LayoutRetry()
            lasers.append((i, pos, direction))
            new_reserved.add(pos)
            new_reserved.update(tiles)
        return lasers, new_reserved

    def _corridor_lasers_vertical(
        self,
        corridor_cols: list[int],
        reserved: set[tuple[int, int]],
    ) -> tuple[list[tuple[int, tuple[int, int], Direction]], set[tuple[int, int]]]:
        """N/S lasers placed on corridor cols from alternating sides."""
        lasers: list[tuple[int, tuple[int, int], Direction]] = []
        new_reserved = set(reserved)
        for i, col in enumerate(corridor_cols):
            if i % 2 == 0:
                pos: tuple[int, int] = (0, col)
                direction = Direction.SOUTH
            else:
                pos = (self.height - 1, col)
                direction = Direction.NORTH
            if pos in new_reserved:
                raise _LayoutRetry()
            tiles = beam_tiles(pos, direction, set(), set(), self.height, self.width)
            if not self._beam_satisfies_span(tiles, pos, direction):
                raise _LayoutRetry()
            lasers.append((i, pos, direction))
            new_reserved.add(pos)
            new_reserved.update(tiles)
        return lasers, new_reserved

    # ------------------------------------------------------------------
    # Wall placement
    # ------------------------------------------------------------------
    def _place_walls(self, reserved: set[tuple[int, int]]) -> list[tuple[int, int]]:
        free = [(r, c) for r in range(self.height) for c in range(self.width) if (r, c) not in reserved]
        if self._wall_cfg.style == "shapes":
            return place_wall_shapes(free, self._wall_cfg.n, self._rng)
        # "individual"
        n = min(self._wall_cfg.n, len(free))
        return self._rng.sample(free, n)
