"""The single, fluent entry point for procedural world generation.

`GeneratorBuilder` is the one public way to describe and run a generation
request. Start from `lle.generate(...)`, chain configuration methods, then call
a terminal (`build`, `take`):

```python
import lle

# Simplest: a solvable world
world = lle.generate(width=10, height=10, n_agents=3).build(seed=0)

# A cooperative world laid out as one lane per agent
world = (
    lle.generate(width=8, height=8, n_agents=2, t_max=30)
    .lanes()
    .cooperative()
    .build(seed=5)
)

# A batch of worlds, generated in parallel
worlds = list(
    lle.generate(width=8, height=8, n_agents=3)
    .walls(4, style="shapes")
    .take(10, n_jobs=4)
)
```
"""

from __future__ import annotations

import math
import random
from dataclasses import replace
from typing import Iterator, Literal, overload

from ..world import World
from .generator import WorldGenerator
from .world_filter import (
    Asymmetric,
    Chained,
    Constraint,
    Convergent,
    Cooperative,
    Divergent,
    Independent,
    Interdependent,
    Predicate,
    Solvable,
)

StartsMode = Literal["random", "edge", "clustered"]
ExitsMode = Literal["random", "edge", "cluster", "opposite"]
ResolvedPlacement = Literal["free", "cross-agent", "cross-cluster"]
LaserPlacement = Literal["auto", "free", "cross-agent", "cross-cluster"]
WallsStyle = Literal["individual", "shapes"]


class GeneratorBuilder:
    """Fluent description of a world-generation request.

    Obtain one from `lle.generate(...)`; never instantiate it directly. Every
    configuration method returns`self` so calls can be chained, and the last
    call of a given category wins (e.g. calling `cooperative()` then `mutual()`
    keeps `mutual`). When two settings would contradict each other, this design
    makes the contradiction unreachable rather than raising at run time.
    """

    def __init__(self, *, width: int = 10, height: int = 10, n_agents: int = 3, t_max: int | Literal["auto"] = "auto"):
        self._width = width
        self._height = height
        self._n_agents = n_agents

        # Layout — defaults to a random scatter.`_layout_explicit` records
        # whether the user pinned a layout, so that an unset layout can be
        # auto-selected from the behavioural filter.
        self._starts: StartsMode = "random"
        self._exits: ExitsMode = "random"
        self._layout_explicit = False

        # Lasers
        self._n_lasers: int | Literal["auto"] = "auto"
        self._laser_placement: LaserPlacement = "auto"
        self._laser_span: int | Literal["any", "across"] = "any"

        # Walls
        self._n_walls: int | Literal["auto"] = "auto"
        self._walls_style: WallsStyle = "individual"

        # Rooms
        self._n_rooms_rows: int = 0
        self._n_rooms_cols: int = 0
        self._door_size: int = 1

        if t_max == "auto":
            t_max = width * height // 2
        self._constraint: Constraint = Constraint(t_max)

    # ------------------------------------------------------------------
    # Layout
    # ------------------------------------------------------------------
    def random(self) -> GeneratorBuilder:
        """Scatter starts and exits anywhere on the grid (the default layout)."""
        return self._set_layout("random", "random")

    def lanes(self) -> GeneratorBuilder:
        """Place agents along one edge with exits on the opposite edge."""
        return self._set_layout("edge", "opposite")

    def clustered(self) -> GeneratorBuilder:
        """Group starts and exits into opposite clusters. Equivalent to
        `self.start("clustered").exits("opposite")`. Check these functions for mor informatin."""
        return self._set_layout("clustered", "opposite")

    def starts(self, mode: StartsMode) -> GeneratorBuilder:
        """
        Set the agent start placement:`"random"`,`"edge"`, or`"clustered"`.

        - `"edge"` places the start positions on one edge of the grid.
        - `"clustered"` places the start positions in an adjacent cluster. Depending on on
            the number of agents, the shape of the cluster can vary (2x1, 1x2, 3x1, 1x3, 2x2, 1x4, 4x1, ...).
        """
        self._starts = mode
        self._layout_explicit = True
        return self

    def exits(self, mode: ExitsMode) -> GeneratorBuilder:
        """
        Set the exit placement:`"random"`,`"edge"`,`"cluster"`, or`"opposite"`.

        - `"edge"` places exits on one edge of the grid (independently of where agents start).
        - `"cluster"` groups exits into an adjacent cluster, mirroring the shape logic of
            `"clustered"` starts (2x1, 1x2, 2x2, …).
        - `"opposite"` mirrors the agent starts to the far side of the grid: the opposite edge
            when starts are`"edge"`, or a cluster on the opposite corner when starts are
           `"clustered"`. Requires `starts` to be`"edge"` or`"clustered"`.
        """
        self._exits = mode
        self._layout_explicit = True
        return self

    def _set_layout(self, starts: StartsMode, exits: ExitsMode) -> GeneratorBuilder:
        self._starts = starts
        self._exits = exits
        self._layout_explicit = True
        return self

    # ------------------------------------------------------------------
    # Lasers and walls
    # ------------------------------------------------------------------
    def lasers(
        self,
        n: int | Literal["auto"] = "auto",
        *,
        placement: LaserPlacement = "auto",
        span: int | Literal["any", "across"] = "any",
    ) -> GeneratorBuilder:
        """Configure the laser sources.

        - `n`: number of sources.`"auto"` picks a random count between 0 and
          the number of agents when no cooperation is required, or a sensible
          minimum when cooperation is required (at least 1, at least 2 for
          chained/mutual/interdependent).
        - `placement`:`"free"` (anywhere valid),`"cross-agent"` (each beam
          crosses all agent lanes; needs `lanes`/`starts("edge")`),
         `"cross-cluster"` (corridor between clusters; needs
          `clustered`), or`"auto"` to derive it from the layout and filter.
        - `span`:`"any"` (2-tile minimum),`"across"` (beam spans the grid),
          or an integer minimum beam length.
        """
        self._n_lasers = n
        self._laser_placement = placement
        self._laser_span = span
        return self

    def walls(
        self,
        n: int | Literal["auto"] = "auto",
        *,
        style: WallsStyle = "individual",
    ) -> GeneratorBuilder:
        """Configure walls: `n` cells (`"auto"` ≈ 10 % of the grid), placed as
        single cells (`"individual"`) or connected`"shapes"`."""
        self._n_walls = n
        self._walls_style = style
        return self

    def rooms(self, n: int = 4, *, door_size: int = 1) -> GeneratorBuilder:
        """Divide the grid into `n` rooms separated by walls with doors.

         Rooms are arranged in a rectangular grid — `n` is factored into
        `rows × cols` as square as possible (e.g. 4 → 2×2, 6 → 2×3, 8 → 2×4).
         Each pair of adjacent rooms is connected by a centered door of `door_size`
         cells carved into the dividing wall. Calling `rooms()` overrides any
         previous `walls()` call.

         ## Parameters
         - `n`: number of rooms (>= 2). Must be factorable into a rectangular
           arrangement.
         - `door_size`: number of cells removed from each wall segment to form a
           door (>= 1). The door is centered on the wall segment facing the room.

         ## Raises
         `ValueError` if `n < 2`, `door_size < 1`, or the grid is too small:
         a`rows × cols` layout requires`height >= 2*rows - 1` and
        `width >= 2*cols - 1` (one cell per room plus one cell per wall).
        """
        if n < 2:
            raise ValueError(f"rooms requires n >= 2, got {n}")
        if door_size < 1:
            raise ValueError(f"door_size must be >= 1, got {door_size}")

        rows = int(math.isqrt(n))
        while rows >= 1:
            if n % rows == 0:
                break
            rows -= 1
        cols = n // rows

        min_height = 2 * rows - 1
        min_width = 2 * cols - 1
        if self._height < min_height:
            raise ValueError(f"{n} rooms ({rows}×{cols} layout) requires height >= {min_height}, got {self._height}")
        if self._width < min_width:
            raise ValueError(f"{n} rooms ({rows}×{cols} layout) requires width >= {min_width}, got {self._width}")

        self._n_rooms_rows = rows
        self._n_rooms_cols = cols
        self._door_size = door_size
        return self

    # ------------------------------------------------------------------
    # Behavioural filter
    # ------------------------------------------------------------------

    def solvable(self) -> GeneratorBuilder:
        """Require only solvability, with no additional cooperation constraint."""
        return self.require(Solvable())

    def independent(self) -> GeneratorBuilder:
        """Require worlds solvable *without* cooperation (no laser blocking needed)."""
        return self.require(Independent())

    def cooperative(self) -> GeneratorBuilder:
        """Require worlds that need cooperation within the configured solver horizon."""
        return self.require(Cooperative())

    def chained(self, length: int) -> GeneratorBuilder:
        """Require chained cooperation of at least `length` help edges."""
        return self.require(Chained(length))

    def mutual(self) -> GeneratorBuilder:
        """Require mutual cooperation: every agent both helps and is helped."""
        return self.interdependent(2)

    def interdependent(self, order: int) -> GeneratorBuilder:
        """Require temporal interdependence of exactly `order` agents."""
        return self.require(Interdependent(order))

    def convergent(self, k: int) -> GeneratorBuilder:
        """Require one beneficiary to receive help from at least `k` distinct agents."""
        return self.require(Convergent(k))

    def divergent(self, k: int) -> GeneratorBuilder:
        """Require one helper to assist at least `k` distinct beneficiaries."""
        return self.require(Divergent(k))

    def asymmetric(self) -> GeneratorBuilder:
        """Require asymmetric cooperation."""
        return self.require(Asymmetric())

    def require(self, predicate: Predicate | Constraint):
        """Constrain generation with an explicit predicate expression.

        Accepts any expression built from predicate atoms and `&`, `|`, `~`.
        Passing a `Constraint` is accepted for advanced callers that want to
        replace the current predicate and horizon together.
        """
        if isinstance(predicate, Constraint):
            self._constraint = predicate
        elif isinstance(predicate, Predicate):
            self._constraint = replace(self._constraint, predicate=predicate)
        else:
            raise TypeError(f"require() expects a Predicate or Constraint, got {type(predicate).__name__}.")
        return self

    def at_least(self, t_min: int):
        """
        Generate world whose solutions require at least `t_min` time steps.
        """
        self._constraint = replace(self._constraint, t_min=t_min)
        return self

    def cap(self, t_max: int):
        """
        Cap the solver horizon to `t_max`, i.e. the generated worlds are guaranteed to ensure the required
        properties for any solution of length at most `t_max`.

        By default, `t_max` is set to width * height // 2.
        """
        self._constraint = replace(self._constraint, t_max=t_max)
        return self

    # ------------------------------------------------------------------
    # Terminals
    # ------------------------------------------------------------------
    @overload
    def build(self, *, seed: int | None = None, n_jobs: Literal[1] = 1) -> World: ...
    @overload
    def build(self, *, seed: int | None = None, max_attempts: int, n_jobs: Literal[1] = 1) -> World | None: ...
    @overload
    def build(self, *, n_jobs: int) -> World: ...
    @overload
    def build(self, *, max_attempts: int, n_jobs: int) -> World | None: ...
    def build(self, *, seed: int | None = None, max_attempts: int | None = None, n_jobs: int = 1):
        """Generate a single world.

        With no `max_attempts` the search runs until it succeeds and always
        returns a `World`. With a bounded `max_attempts` it returns the world,
        or`None` if the budget is exhausted.

        `seed` is only accepted when `n_jobs=1`; it has no meaningful effect
        across parallel workers because concurrency can yield different results
        with the same seed.
        """
        if n_jobs < 1:
            raise ValueError("n_jobs must be >= 1")
        if n_jobs > 1 and seed is not None:
            raise ValueError(
                "Seed is only accepted when n_jobs=1 because parallel workers may yield different results with the same seed due to concurrency."
            )
        generator = self._make_generator()
        if n_jobs == 1:
            return generator.generate(max_attempts=max_attempts, seed=seed)
        try:
            return next(generator.generate_n(1, n_jobs=n_jobs, seed=seed, max_attempts=max_attempts))
        except StopIteration:
            return None

    def take(
        self,
        n: int,
        *,
        n_jobs: int | Literal["auto"] = "auto",
        seed: int | None = None,
        max_attempts: int | None = None,
        progress: bool = True,
    ) -> Iterator[World]:
        """Generate up to `n` worlds, yielding each as it is produced.

        - `n_jobs`: parallel workers;`"auto"` uses all CPUs but one.
        - `seed`: fixed RNG seed — only accepted when `n_jobs=1` because parallel workers may yield different results with the same seed due to concurrency.
        - `max_attempts`: total attempt budget; the stream may be shorter than
          `n` if the budget runs out.
        - `progress`: show a progress bar.
        """
        resolved_jobs = self._resolve_n_jobs(n_jobs)
        return self._make_generator().generate_n(
            n=n,
            n_jobs=resolved_jobs,
            seed=seed,
            max_attempts=max_attempts,
            quiet=not progress,
        )

    # ------------------------------------------------------------------
    # Compilation
    # ------------------------------------------------------------------

    def _make_generator(self) -> WorldGenerator:
        """Compile the accumulated configuration into a concrete generator."""
        starts, exits = self._resolve_layout()
        placement = self._resolve_placement(starts)
        n_lasers = self._resolve_n_lasers(placement)
        return WorldGenerator(
            width=self._width,
            height=self._height,
            n_agents=self._n_agents,
            starts=starts,
            exits=exits,
            n_lasers=n_lasers,
            laser_placement=placement,
            laser_span=self._laser_span,
            n_walls=self._n_walls,
            walls_style=self._walls_style,
            n_rooms_rows=self._n_rooms_rows,
            n_rooms_cols=self._n_rooms_cols,
            door_size=self._door_size,
            constraint=self._constraint,
        )

    def _resolve_layout(self) -> tuple[StartsMode, ExitsMode]:
        """Pick a layout. An explicit layout is honoured; otherwise one is chosen
        to suit the behavioural constraint (cooperation favours opposite-side layouts)."""
        if self._layout_explicit:
            return self._starts, self._exits
        # requirements = self._constraint.requirements
        # if requirements.mutual:
        #     return "clustered", "opposite"
        # if requirements.cooperation:
        #     return "edge", "opposite"
        return self._starts, self._exits

    def _resolve_placement(self, starts: StartsMode) -> ResolvedPlacement:
        """Derive a laser placement from the layout when left on`"auto"`."""
        if self._laser_placement != "auto":
            return self._laser_placement
        # if self._constraint.requirements.cooperation:
        #     if starts == "clustered":
        #         return "cross-cluster"
        #     if starts == "edge":
        #         return "cross-agent"
        return "free"

    def _resolve_n_lasers(self, placement: ResolvedPlacement) -> int:
        if self._n_lasers != "auto":
            return self._n_lasers
        requirements = self._constraint.requirements
        if requirements.min_lasers > 0:
            return min(self._n_agents, max(requirements.min_lasers, self._n_agents - 1))
        # if requirements.cooperation or placement in ("cross-agent", "cross-cluster"):
        #     return min(self._n_agents, max(1, self._n_agents - 1))
        return random.randint(0, self._n_agents)

    def _resolve_n_jobs(self, n_jobs: int | Literal["auto"]) -> int:
        if n_jobs != "auto":
            return n_jobs
        from multiprocessing import cpu_count

        return max(1, cpu_count() - 1)
