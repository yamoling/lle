"""Shared machinery for internal world generators.

Generators sample candidate layouts, build a `World`, and optionally filter
it by solvability or cooperation profile. Subclasses only have to implement
`_make_candidate_layout`.
"""

from __future__ import annotations

import multiprocessing as mp
import random
import sys
from abc import ABC, abstractmethod

from tqdm import tqdm

from ..world import World
from ._candidates import CandidateLayout
from ._world_builder import WorldBuilder
from .world_filter import WorldFilter


class _LayoutRetry(Exception):
    """Raised by a generator when sampling produced an unusable layout."""


class Generator(ABC):
    def __init__(
        self,
        *,
        width: int,
        height: int,
        n_agents: int = 2,
        n_lasers: int = 0,
        n_walls: int,
        world_filter: WorldFilter | None = None,
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

        if n_lasers < 0:
            raise ValueError(f"lasers must be >= 0. Got {n_lasers}")
        if n_lasers > n_agents:
            raise ValueError(f"lasers must be <= agents (one laser source per colour). Got lasers={n_lasers}, agents={n_agents}.")
        self.n_lasers = n_lasers
        self.n_walls = n_walls
        if self.n_walls < 0:
            raise ValueError(f"num_walls must be >= 0. Got {self.n_walls}")
        if self.n_walls >= (self.area / 2):
            raise ValueError(f"num_walls must be < size/2. Got num_walls={self.n_walls}, size={self.area}")

        self.world_filter = world_filter

        total_needed = (2 * self.agents) + self.n_walls + self.n_lasers
        if total_needed > self.area:
            raise ValueError(f"layout requires {total_needed} unique cells, but grid has only {self.area}")
        self._rng = random.Random()

    @abstractmethod
    def _make_candidate_layout(self) -> CandidateLayout: ...

    def _build_world(self, layout: CandidateLayout):
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

    def _accept_world(self, world: World) -> bool:
        if self.world_filter is None:
            return True
        return self.world_filter.is_satisfied_by(world)

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

    def generate(self, max_attempts: int | None, seed: int | None = None):
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

    def generate_n(self, n: int, n_jobs: int, seed: int | None = None, max_attempts: int | None = None, quiet: bool = False):
        if seed is not None:
            self._rng.seed(seed)
        if n_jobs < 1:
            raise ValueError("Invalid argument in 'generate_n': n_jobs must be >=1")
        show_attemps = max_attempts is not None
        if max_attempts is None:
            max_attempts = sys.maxsize
        n_generated = 0
        try:
            with mp.Pool(n_jobs) as pool, tqdm(total=n, disable=quiet) as pbar:
                # Worker seeds are 0, 1, 2, ...
                results = pool.imap_unordered(self._try_generate, range(max_attempts))
                for i, result in enumerate(results):
                    if show_attemps and not quiet:
                        budget_percent = 100 * (i + 1) / max_attempts
                        pbar.set_description(f"{budget_percent:.2f}% budget elapsed")
                    if result is not None:
                        n_generated += 1
                        pbar.update(1)
                        yield result
                        if n_generated >= n:
                            pool.terminate()
                            return
        except ConnectionResetError as e:
            raise RuntimeError(
                "Error in the processing pool. Do you have an \"if __name__ == '__main__':\" guard around the entry of the main script?"
            ) from e
        return
