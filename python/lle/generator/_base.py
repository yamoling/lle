"""Private base class for all internal generators.

Adds a `cooperative` flag (off by default) that gates a post-filter using
is_cooperative-equivalent logic (standard SAT solvable and strict-laser
UNSAT). Subclasses implement `_make_candidate_layout`.
"""
from __future__ import annotations

import random
from abc import ABC, abstractmethod

from lle import World

from ._candidates import CandidateLayout
from ._world_builder import WorldBuilder
from ..solver._world_solver import LaserMode, WorldSolver


class _LayoutRetry(Exception):
    """Raised by a generator when sampling produced an unusable layout."""


class _BaseGenerator(ABC):
    def __init__(
        self,
        *,
        size: tuple[int, int],
        agents: int = 2,
        lasers: int = 0,
        cooperative: bool = False,
        num_walls: int | None = None,
        t_max: int | None = None,
        seed: int | None = None,
        max_attempts: int = 10_000,
    ):
        self.rows, self.cols = size
        if self.rows < 1 or self.cols < 1:
            raise ValueError(f"grid dimensions must be >= 1. Got size={size}")
        self.area = self.rows * self.cols

        if agents < 1:
            raise ValueError(f"agents must be >= 1. Got {agents}")
        self.agents = agents

        if cooperative and agents < 2:
            raise ValueError(
                "cooperative=True requires agents >= 2 "
                "(no cooperation possible with a single agent)."
            )
        self.cooperative = cooperative

        if lasers < 0:
            raise ValueError(f"lasers must be >= 0. Got {lasers}")
        if cooperative and not (1 <= lasers <= agents):
            raise ValueError(
                f"cooperative=True requires lasers in [1, agents]; "
                f"got lasers={lasers}, agents={agents}."
            )
        if lasers > agents:
            raise ValueError(
                f"lasers must be <= agents (one laser source per colour). "
                f"Got lasers={lasers}, agents={agents}."
            )
        self.lasers = lasers

        self.num_walls = (self.area // 10) if num_walls is None else num_walls
        if self.num_walls < 0:
            raise ValueError(f"num_walls must be >= 0. Got {self.num_walls}")
        if self.num_walls >= (self.area / 2):
            raise ValueError(
                f"num_walls must be < size/2. Got num_walls={self.num_walls}, "
                f"size={self.area}"
            )

        self.t_max = (self.area // 2) if t_max is None else t_max
        if self.t_max < 0:
            raise ValueError(f"t_max must be >= 0. Got {self.t_max}")

        if max_attempts < 1:
            raise ValueError(f"max_attempts must be >= 1. Got {max_attempts}")
        self.max_attempts = max_attempts

        total_needed = (2 * self.agents) + self.num_walls + self.lasers
        if total_needed > self.area:
            raise ValueError(
                f"layout requires {total_needed} unique cells, "
                f"but grid has only {self.area}"
            )

        self._rng = random.Random(seed)
        self.last_attempts = 0

    @abstractmethod
    def _make_candidate_layout(self) -> CandidateLayout: ...

    def _build_world(self, layout: CandidateLayout) -> World:
        b = WorldBuilder(self.cols, self.rows)
        for agent_id, pos in enumerate(layout.agents):
            b.add_agent(agent_id, pos)
        for pos in layout.exits:
            b.add_exit(pos)
        for pos in layout.walls:
            b.add_wall(pos)
        for owner, pos, direction in layout.lasers:
            b.add_laser(owner, pos, direction)
        return b.build()

    def _is_satisfiable(self, world: World, t: int) -> bool:
        world.reset()
        sat, _ = WorldSolver(world, T_MAX=t).solve()
        return bool(sat)

    def _is_cooperative(self, world: World) -> bool:
        world.reset()
        sat, _ = WorldSolver(world, T_MAX=self.t_max, laser_mode=LaserMode.STRICT).solve()
        return not bool(sat)

    def _accept_world(self, world: World) -> bool:
        if not self._is_satisfiable(world, self.t_max):
            return False
        if self.cooperative and not self._is_cooperative(world):
            return False
        return True

    def generate(self) -> World:
        self.last_attempts = 0
        for attempt in range(1, self.max_attempts + 1):
            self.last_attempts = attempt
            try:
                layout = self._make_candidate_layout()
            except _LayoutRetry:
                continue
            if layout is None:
                continue
            try:
                world = self._build_world(layout)
            except Exception:
                continue
            try:
                if self._accept_world(world):
                    return world
            except Exception:
                continue
        raise RuntimeError(
            f"{type(self).__name__}: could not generate a valid world in "
            f"{self.max_attempts} attempts (cooperative={self.cooperative})."
        )
