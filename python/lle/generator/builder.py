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
    lle.generate(width=8, height=8, n_agents=2)
    .lanes()
    .cooperative(t_max=30)
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

from typing import Literal, overload

from ..world import World
from .generator import WorldGenerator
from .world_filter import Chained, Cooperative, Independent, Interdependent, Mutual, Solvable, WorldFilter

StartsMode = Literal["random", "edge", "clustered"]
ExitsMode = Literal["random", "edge", "cluster", "opposite"]
ResolvedPlacement = Literal["free", "cross-agent", "cross-cluster"]
LaserPlacement = Literal["auto", "free", "cross-agent", "cross-cluster"]
WallsStyle = Literal["individual", "shapes"]


class GeneratorBuilder:
    """Fluent description of a world-generation request.

    Obtain one from `lle.generate(...)`; never instantiate it directly. Every
    configuration method returns ``self`` so calls can be chained, and the last
    call of a given category wins (e.g. calling `cooperative()` then `mutual()`
    keeps `mutual`). When two settings would contradict each other, this design
    makes the contradiction unreachable rather than raising at run time.
    """

    def __init__(self, *, width: int = 10, height: int = 10, n_agents: int = 3, t_max: int | Literal["auto"] = "auto"):
        self._width = width
        self._height = height
        self._n_agents = n_agents

        # Layout — defaults to a random scatter. ``_layout_explicit`` records
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

        # Behavioural filter — single source of truth for t_max/t_min and the
        # cooperation constraint. Initialised to "any solvable world".
        if t_max == "auto":
            t_max = width * height // 2
        self._world_filter: WorldFilter = Solvable(t_max)

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
        """Group starts and exits into opposite clusters (Level-6 style)."""
        return self._set_layout("clustered", "opposite")

    def starts(self, mode: StartsMode) -> GeneratorBuilder:
        """Set the agent start placement: ``"random"``, ``"edge"``, or ``"clustered"``."""
        self._starts = mode
        self._layout_explicit = True
        return self

    def exits(self, mode: ExitsMode) -> GeneratorBuilder:
        """Set the exit placement: ``"random"``, ``"edge"``, ``"cluster"``, or ``"opposite"``."""
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

        - `n`: number of sources; ``"auto"`` picks a sensible count (one helper
          per other agent when cooperation is required, otherwise none).
        - `placement`: ``"free"`` (anywhere valid), ``"cross-agent"`` (each beam
          crosses all agent lanes; needs `lanes`/`starts("edge")`),
          ``"cross-cluster"`` (corridor between clusters; needs
          `clustered`), or ``"auto"`` to derive it from the layout and filter.
        - `span`: ``"any"`` (2-tile minimum), ``"across"`` (beam spans the grid),
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
        """Configure walls: `n` cells (``"auto"`` ≈ 10 % of the grid), placed as
        single cells (``"individual"``) or connected ``"shapes"``."""
        self._n_walls = n
        self._walls_style = style
        return self

    # ------------------------------------------------------------------
    # Behavioural filter
    # ------------------------------------------------------------------

    def solvable(self) -> GeneratorBuilder:
        """Accept any solvable world (the default constraint)."""
        self._world_filter = Solvable(self._world_filter.t_max, self._world_filter.t_min)
        return self

    def independent(self) -> GeneratorBuilder:
        """Require worlds solvable *without* cooperation (no laser blocking needed)."""
        self._world_filter = Independent(self._world_filter.t_max, self._world_filter.t_min)
        return self

    def cooperative(
        self,
        t_max: int | None = None,
        t_min: int | None = None,
    ) -> GeneratorBuilder:
        """Require worlds that *need* cooperation within `t_max` steps."""
        self._world_filter = Cooperative(
            t_max if t_max is not None else self._world_filter.t_max,
            t_min if t_min is not None else self._world_filter.t_min,
        )
        return self

    def chained(
        self,
        t_max: int | None = None,
        t_min: int | None = None,
    ) -> GeneratorBuilder:
        """Require *chained* cooperation: a helped b, then b helped c (chain length ≥ 2)."""
        self._world_filter = Chained(
            t_max if t_max is not None else self._world_filter.t_max,
            t_min if t_min is not None else self._world_filter.t_min,
        )
        return self

    def mutual(
        self,
        t_max: int | None = None,
        t_min: int | None = None,
    ) -> GeneratorBuilder:
        """Require *mutual* cooperation: every agent both helps and is helped."""
        self._world_filter = Mutual(
            t_max if t_max is not None else self._world_filter.t_max,
            t_min if t_min is not None else self._world_filter.t_min,
        )
        return self

    def interdependent(
        self,
        k: int = 2,
        t_max: int | None = None,
        t_min: int | None = None,
    ) -> GeneratorBuilder:
        """Require *temporal interdependence* at level ``k``: a temporal cycle of order >= ``k``
        is forced by every solution within ``t_max``.  ``k=2`` recovers mutual cooperation."""
        self._world_filter = Interdependent(
            t_max if t_max is not None else self._world_filter.t_max,
            t_min if t_min is not None else self._world_filter.t_min,
            k=k,
        )
        return self

    def require(self, filter: WorldFilter):
        """Constrain generation with an explicit `WorldFilter` (escape hatch for
        custom or future filters). Overrides the named filter shortcuts."""
        self._world_filter = filter
        return self

    def at_least(self, t_min: int):
        """
        Generate world whose solutions require at least `t_min` time steps.
        """
        self._world_filter.t_min = t_min
        return self

    def cap(self, t_max: int):
        """
        Cap the solver horizon to `t_max`, i.e. the generated worlds are guaranteed to ensure the required
        properties for any solution of length at most `t_max`.

        By default, `t_max` is set to width * height // 2.
        """
        self._world_filter.t_max = t_max
        return self

    # ------------------------------------------------------------------
    # Terminals
    # ------------------------------------------------------------------
    @overload
    def build(self, *, seed: int | None = None, n_jobs: int = 1) -> World: ...
    @overload
    def build(self, *, seed: int | None = None, max_attempts: int, n_jobs: int = 1) -> World | None: ...
    def build(self, *, seed: int | None = None, max_attempts: int | None = None, n_jobs: int = 1):
        """Generate a single world.

        With no `max_attempts` the search runs until it succeeds and always
        returns a `World`. With a bounded `max_attempts` it returns the world,
        or ``None`` if the budget is exhausted.
        """
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
    ):
        """Generate up to `n` worlds, yielding each as it is produced.

        - `n_jobs`: parallel workers; ``"auto"`` uses all CPUs but one.
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
            filter=self._world_filter,
        )

    def _resolve_layout(self) -> tuple[StartsMode, ExitsMode]:
        """Pick a layout. An explicit layout is honoured; otherwise one is chosen
        to suit the behavioural filter (cooperation favours opposite-side layouts)."""
        if self._layout_explicit:
            return self._starts, self._exits
        if self._world_filter.requires_mutual_cooperation:
            return "clustered", "opposite"
        if self._world_filter.requires_cooperation:
            return "edge", "opposite"
        return self._starts, self._exits

    def _resolve_placement(self, starts: StartsMode) -> ResolvedPlacement:
        """Derive a laser placement from the layout when left on ``"auto"``."""
        if self._laser_placement != "auto":
            return self._laser_placement
        if self._world_filter.requires_cooperation:
            if starts == "clustered":
                return "cross-cluster"
            if starts == "edge":
                return "cross-agent"
        return "free"

    def _resolve_n_lasers(self, placement: ResolvedPlacement) -> int:
        if self._n_lasers != "auto":
            return self._n_lasers
        k = self._world_filter.requires_interdependence_order
        if k > 0:
            return min(self._n_agents, k)
        if self._world_filter.requires_chained_cooperation:
            return min(self._n_agents, max(2, self._n_agents - 1))
        if self._world_filter.requires_cooperation or placement in ("cross-agent", "cross-cluster"):
            return min(self._n_agents, max(1, self._n_agents - 1))
        return 0

    def _resolve_n_jobs(self, n_jobs: int | Literal["auto"]) -> int:
        if n_jobs != "auto":
            return n_jobs
        from multiprocessing import cpu_count

        return max(1, cpu_count() - 1)
