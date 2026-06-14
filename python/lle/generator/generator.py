"""Customizable layout generator with explicit control over every placement decision."""

from __future__ import annotations

import multiprocessing as mp
import random
import sys
from dataclasses import dataclass
from typing import Literal

from tqdm import tqdm

from ..world import World
from .candidates import CandidateLayout
from .placements import (
    LayoutRetry,
    PlacementCtx,
    place_agents,
    place_exits,
    place_lasers,
    place_walls,
)
from .world_builder import WorldBuilder
from .world_filter import WorldFilter


@dataclass
class AgentConfig:
    mode: Literal["random", "edge", "clustered"]


@dataclass
class ExitConfig:
    mode: Literal["random", "edge", "cluster", "opposite"]


@dataclass
class LaserConfig:
    n: int
    placement: Literal["free", "cross-agent", "cross-cluster"]
    span: int | Literal["any", "across"]


@dataclass
class WallConfig:
    n: int
    style: Literal["individual", "shapes"]


class WorldGenerator:
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

        if width < 1:
            raise ValueError(f"Grid width must be >= 1. Got {width}")
        if height < 1:
            raise ValueError(f"Grid height must be >= 1. Got {height}")
        self.height, self.width = height, width
        area = height * width

        if n_agents < 1:
            raise ValueError(f"agents must be >= 1. Got {n_agents}")
        self.agents = n_agents

        if n_lasers < 0:
            raise ValueError(f"lasers must be >= 0. Got {n_lasers}")
        if n_lasers > n_agents:
            raise ValueError(f"lasers must be <= agents (one laser source per colour). Got lasers={n_lasers}, agents={n_agents}.")
        self.n_lasers = n_lasers

        if filter is not None:
            if filter.requires_cooperation:
                if n_agents < 2:
                    raise ValueError("Cooperative worlds require at least 2 agents.")
                if n_lasers == 0:
                    raise ValueError("Cooperative worlds are impossible with 0 lasers.")
            if filter.requires_chained_cooperation and n_lasers < 2:
                raise ValueError("Chained cooperation requires at least 2 lasers.")

        resolved_n_walls = (width * height) // 10 if n_walls == "auto" else n_walls
        self.n_walls = resolved_n_walls
        if self.n_walls < 0:
            raise ValueError(f"num_walls must be >= 0. Got {self.n_walls}")
        if self.n_walls >= (area / 2):
            raise ValueError(f"num_walls must be < size/2. Got num_walls={self.n_walls}, size={area}")

        total_needed = (2 * self.agents) + self.n_walls + self.n_lasers
        if total_needed > area:
            raise ValueError(f"layout requires {total_needed} unique cells, but grid has only {area}")

        self.world_filter = filter
        self._rng = random.Random()

        self._agent_cfg = AgentConfig(mode=starts)
        self._exit_cfg = ExitConfig(mode=exits)
        self._laser_cfg = LaserConfig(n=n_lasers, placement=laser_placement, span=laser_span)
        self._wall_cfg = WallConfig(n=resolved_n_walls, style=walls_style)

    # ------------------------------------------------------------------
    # Layout assembly
    # ------------------------------------------------------------------

    def _make_candidate_layout(self) -> CandidateLayout:
        ctx = PlacementCtx()
        agents, reserved = place_agents(self._agent_cfg.mode, self.agents, self.height, self.width, self._rng, ctx)
        exits, reserved = place_exits(self._exit_cfg.mode, self.agents, self.height, self.width, self._rng, reserved, ctx)
        lasers, reserved = place_lasers(
            self._laser_cfg.n,
            self._laser_cfg.placement,
            self._laser_cfg.span,
            self.agents,
            self.height,
            self.width,
            self._rng,
            reserved,
            ctx,
        )
        walls = place_walls(self._wall_cfg.n, self._wall_cfg.style, reserved, self.height, self.width, self._rng)
        layout = CandidateLayout(self.height, self.width, agents=agents, exits=exits, walls=walls, lasers=lasers)
        if not layout.is_geometry_valid():
            raise LayoutRetry()
        return layout

    def _build_world(self, layout: CandidateLayout) -> World:
        b = WorldBuilder(self.width, self.height)
        for agent_id, pos in enumerate(layout.agents):
            b.add_agent(agent_id, pos)
        for pos in layout.exits:
            b.add_exit(pos)
        for pos in layout.walls:
            b.add_wall(pos)
        for owner, pos, direction in layout.lasers:
            b.add_laser(owner, pos, direction)
        return b.build()

    def _accept_world(self, world: World) -> bool:
        if self.world_filter is None:
            return True
        return self.world_filter.is_satisfied_by(world)

    # ------------------------------------------------------------------
    # Generation loop
    # ------------------------------------------------------------------

    def _try_generate(self, seed: int | None) -> World | None:
        if seed is not None:
            self._rng.seed(seed)
        try:
            layout = self._make_candidate_layout()
        except LayoutRetry:
            return None
        if layout is None:
            return None
        try:
            world = self._build_world(layout)
        except Exception:
            return None
        try:
            if self._accept_world(world):
                return world
        except Exception:
            return None
        return None

    def generate(self, max_attempts: int | None, seed: int | None = None) -> World | None:
        if seed is not None:
            self._rng.seed(seed)
        if max_attempts is None:
            max_attempts = sys.maxsize
        if max_attempts < 1:
            raise ValueError(f"max_attempts must be >= 1. Got {max_attempts}")
        for _ in range(max_attempts):
            maybe_world = self._try_generate(None)
            if maybe_world is not None:
                return maybe_world
        return None

    def _generate_n_single(self, n: int, max_attempts: int):
        n_generated = 0
        for i in range(max_attempts):
            result = self._try_generate(None)
            yield i, result
            if result is not None:
                n_generated += 1
                if n_generated >= n:
                    return

    def _generate_n_multi(self, n: int, n_jobs: int, max_attempts: int):
        n_generated = 0
        try:
            with mp.Pool(n_jobs) as pool:
                results = pool.imap_unordered(self._try_generate, range(max_attempts))
                for i, result in enumerate(results):
                    yield i, result
                    if result is not None:
                        n_generated += 1
                        if n_generated >= n:
                            pool.terminate()
                            return
        except ConnectionResetError as e:
            raise RuntimeError(
                "Error in the processing pool. Do you have an \"if __name__ == '__main__':\" guard around the entry of the main script?"
            ) from e

    def generate_n(self, n: int, n_jobs: int, seed: int | None = None, max_attempts: int | None = None, quiet: bool = False):
        if seed is not None:
            self._rng.seed(seed)
        if n_jobs < 1:
            raise ValueError("Invalid argument in 'generate_n': n_jobs must be >=1")
        show_attempts = max_attempts is not None
        if max_attempts is None:
            max_attempts = sys.maxsize
        if n_jobs == 1:
            generator = self._generate_n_single(n, max_attempts)
        else:
            generator = self._generate_n_multi(n, n_jobs, max_attempts)
        with tqdm(total=n, disable=quiet) as pbar:
            for i, result in generator:
                if show_attempts and not quiet:
                    budget_percent = 100 * (i + 1) / max_attempts
                    pbar.set_description(f"{budget_percent:.2f}% budget elapsed")
                if result is not None:
                    pbar.update(1)
                    yield result
