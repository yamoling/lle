"""Private base class for all internal generators.

Adds a `cooperative` flag (off by default) that gates a post-filter using
is_cooperative-equivalent logic (standard SAT solvable and strict-laser
UNSAT). Subclasses implement `_make_candidate_layout`.
"""

from __future__ import annotations

import multiprocessing as mp
import random
import sys
from abc import ABC, abstractmethod

from tqdm import tqdm

from lle import World

from ..solver.world_solver import LaserMode, WorldSolver
from ._candidates import CandidateLayout
from ._world_builder import WorldBuilder


class _LayoutRetry(Exception):
    """Raised by a generator when sampling produced an unusable layout."""


class _BaseGenerator(ABC):
    def __init__(
        self,
        *,
        width: int,
        height: int,
        n_agents: int = 2,
        n_lasers: int = 0,
        cooperative: bool = False,
        n_walls: int | None = None,
        t_max: int | None = None,
    ):
        if width < 1:
            raise ValueError(f"Grid width must be >= 1. Got {width}")
        if height < 1:
            raise ValueError(f"Grid height must be >= 1. Got {height}")
        self.rows, self.cols = height, width
        self.area = self.rows * self.cols

        if n_agents < 1:
            raise ValueError(f"agents must be >= 1. Got {n_agents}")
        self.agents = n_agents

        if cooperative and n_agents < 2:
            raise ValueError("cooperative=True requires agents >= 2 (no cooperation possible with a single agent).")
        self.cooperative = cooperative

        if n_lasers < 0:
            raise ValueError(f"lasers must be >= 0. Got {n_lasers}")
        if cooperative and not (1 <= n_lasers <= n_agents):
            raise ValueError(f"cooperative=True requires lasers in [1, agents]; got lasers={n_lasers}, agents={n_agents}.")
        if n_lasers > n_agents:
            raise ValueError(f"lasers must be <= agents (one laser source per colour). Got lasers={n_lasers}, agents={n_agents}.")
        self.n_lasers = n_lasers

        self.n_walls = (self.area // 10) if n_walls is None else n_walls
        if self.n_walls < 0:
            raise ValueError(f"num_walls must be >= 0. Got {self.n_walls}")
        if self.n_walls >= (self.area / 2):
            raise ValueError(f"num_walls must be < size/2. Got num_walls={self.n_walls}, size={self.area}")

        self.t_max = (self.area // 2) if t_max is None else t_max
        if self.t_max < 0:
            raise ValueError(f"t_max must be >= 0. Got {self.t_max}")

        total_needed = (2 * self.agents) + self.n_walls + self.n_lasers
        if total_needed > self.area:
            raise ValueError(f"layout requires {total_needed} unique cells, but grid has only {self.area}")
        self._rng = random.Random()

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
        sat, _ = WorldSolver(world, t_max=t).solve()
        return bool(sat)

    def _strict_laser_unsat(self, world: World) -> bool:
        world.reset()
        sat, _ = WorldSolver(world, t_max=self.t_max, laser_mode=LaserMode.STRICT).solve()
        return not bool(sat)

    def _accept_world(self, world: World) -> bool:
        if not self._is_satisfiable(world, self.t_max):
            return False
        if self.cooperative and not self._strict_laser_unsat(world):
            return False
        return True

    def _try_generate(self, seed: int | None):
        if seed is not None:
            self._rng.seed(seed)
        try:
            layout = self._make_candidate_layout()
        except _LayoutRetry:
            return
        if layout is None:
            return
        try:
            world = self._build_world(layout)
        except Exception:
            return
        try:
            if self._accept_world(world):
                return world
        except Exception:
            return

    def generate(self, max_attempts: int | None, seed: int | None = None) -> World | None:
        if max_attempts is None:
            max_attempts = sys.maxsize
        if max_attempts < 1:
            raise ValueError(f"max_attempts must be >= 1. Got {max_attempts}")
        if seed is not None:
            self._rng.seed(seed)
        for _ in range(max_attempts):
            maybe_world = self._try_generate(None)
            if maybe_world is not None:
                return maybe_world
        return None

    def generate_n(self, n: int, n_jobs: int, seed: int | None = None) -> list[World]:
        """"""
        if seed is not None:
            self._rng.seed(seed)
        if n_jobs < 1:
            raise ValueError("Invalid argument in 'generate_n': n_jobs must be >=1")
        worlds = list[World]()
        with mp.Pool(n_jobs) as pool, tqdm(total=n) as pbar:
            # Worker seeds are 0, 1, 2, ...
            results = pool.imap_unordered(self._try_generate, range(sys.maxsize))
            for result in results:
                if result is not None:
                    pbar.update(1)
                    worlds.append(result)
                    if len(worlds) >= n:
                        pool.terminate()
                        return worlds
        return worlds
