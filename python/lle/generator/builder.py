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
from typing import Any, Iterator, Literal, Sequence, overload

from ..world import World
from .generator import WorldGenerator
from .layered import ActuatorChoice, LayerConfig, LayeredGenerator
from .world_filter import (
    Asymmetric,
    Constraint,
    Convergent,
    Cooperative,
    Divergent,
    Independent,
    Interdependent,
    Predicate,
    Sequential,
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

        # Gems and boxes
        self._n_gems = 0
        self._n_boxes = 0

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

        # Layers — a single layer unless `layers()` is called.
        self._n_layers = 1
        self._start_layers: list[int] | Literal["random"] = "random"
        self._max_layer_changes: int | None = None
        self._extra_exits = 0
        self._cross_layer_actuators = False
        self._actuators: tuple[ActuatorChoice, ...] = ("button",)
        self._restricted_lifts = 0.0
        self._layer_overrides: dict[int, dict[str, Any]] = {}

        # An automatic horizon grows with the number of layers, see `_make_layered_generator`.
        self._auto_t_max = t_max == "auto"
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
        `self.starts("clustered").exits("opposite")`. See `starts()` and `exits()` for details."""
        return self._set_layout("clustered", "opposite")

    def starts(self, mode: StartsMode) -> GeneratorBuilder:
        """
        Set the agent start placement: `"random"`, `"edge"`, or `"clustered"`.

        - `"edge"` places the start positions on one edge of the grid.
        - `"clustered"` places the start positions in an adjacent cluster. Depending on
            the number of agents, the shape of the cluster can vary (2x1, 1x2, 3x1, 1x3, 2x2, 1x4, 4x1, ...).
        """
        self._starts = mode
        self._layout_explicit = True
        return self

    def exits(self, mode: ExitsMode) -> GeneratorBuilder:
        """
        Set the exit placement: `"random"`, `"edge"`, `"cluster"`, or `"opposite"`.

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
    # Gems, lasers, and walls
    # ------------------------------------------------------------------
    def gems(self, n: int) -> GeneratorBuilder:
        """Place exactly `n` gems on distinct free cells."""
        self._n_gems = n
        return self

    def boxes(self, n: int) -> GeneratorBuilder:
        """Place exactly `n` movable boxes on distinct free cells.

        A box blocks lasers of any colour, but pushing one is never help: boxes can only let agents
        avoid cooperation, never force it.
        """
        self._n_boxes = n
        return self

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
          sequential/mutual/interdependent
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

    def layers(
        self,
        n: int,
        *,
        max_changes: int | None = None,
        extra_exits: int = 0,
        cross_layer_actuators: bool = False,
        restricted_lifts: float = 0.0,
    ) -> GeneratorBuilder:
        """Generate `n` layers linked by lifts, layer 0 being the bottom one.

        Every agent follows a random journey through the layers, one layer up or down at a time,
        and the lifts, their actuators (see `actuators()`) and the exits are placed along the
        journeys. The other settings (`lasers`, `walls`, `gems`, `boxes`, `actuators`) apply to every
        layer; `layer()` overrides them for one layer. Filters (`solvable()`, `cooperative()`, ...)
        apply to the whole level, whose shortest plan must also visit every layer. Only the random
        layout is supported.

        - `max_changes`: most layer changes in one journey (default: `n`).
        - `extra_exits`: exits added on random layers on top of one exit per agent.
        - `cross_layer_actuators`: also put a copy of a lift's actuators on the layer it lands on, so
          that an agent who already rode it can activate it for the others.
        - `restricted_lifts`: probability that a lift with a single rider only accepts the colour of
          that rider (its actuators then only accept the colour of another agent visiting their layer).
        """
        if n < 1:
            raise ValueError(f"layers requires n >= 1, got {n}")
        self._n_layers = n
        self._max_layer_changes = max_changes
        self._extra_exits = extra_exits
        self._cross_layer_actuators = cross_layer_actuators
        self._restricted_lifts = restricted_lifts
        return self

    def actuators(self, *kinds: ActuatorChoice) -> GeneratorBuilder:
        """The actuators of every lift (requires `layers()`), one per kind given: `"button"` (the
        default), `"lever"`, `"plate"` (pressure plate), or `"random"` to draw a kind for each lift.

        Giving several kinds puts several actuators on each lift, any of which moves it:
        `actuators("lever", "button")` gives every lift a lever and a button. `layer(k, actuators=...)`
        overrides them for the lifts leaving layer `k`.

        ```python
        lle.generate(width=6, height=6, n_agents=3).layers(3).actuators("random").layer(0, actuators=["plate"])
        ```
        """
        self._actuators = kinds
        return self

    def start_layers(self, layers: Sequence[int] | Literal["random"]) -> GeneratorBuilder:
        """The layer each agent starts on, or `"random"` (the default)."""
        self._start_layers = layers if layers == "random" else list(layers)
        return self

    def layer(
        self,
        k: int,
        *,
        lasers: int | None = None,
        walls: int | Literal["auto"] | None = None,
        gems: int | None = None,
        boxes: int | None = None,
        actuators: Sequence[ActuatorChoice] | None = None,
    ) -> GeneratorBuilder:
        """Override the number of lasers, walls, gems or boxes of layer `k`, or the actuators of the
        lifts leaving it (see `actuators()`). Requires `layers()`."""
        overrides = {"lasers": lasers, "walls": walls, "gems": gems, "boxes": boxes, "actuators": actuators}
        self._layer_overrides.setdefault(k, {}).update({key: v for key, v in overrides.items() if v is not None})
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

    def sequential(self, length: int) -> GeneratorBuilder:
        """Require sequential cooperation of at least `length` help edges."""
        return self.require(Sequential(length))

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
            self._auto_t_max = False
        elif isinstance(predicate, Predicate):
            self._constraint = replace(self._constraint, predicate=predicate)
        else:
            raise TypeError(f"require() expects a Predicate or Constraint, got {type(predicate).__name__}.")
        return self

    def at_least(self, path_length: int):
        """Reject worlds with a solution shorter than `t_min` time steps."""
        self._constraint = replace(self._constraint, min_solution_length=path_length)
        return self

    def cap(self, t_max: int):
        """
        Cap the solver horizon to `t_max`, i.e. the generated worlds are guaranteed to ensure the required
        properties for any solution of length at most `t_max`.

        By default, `t_max` is set to width * height // 2.
        """
        self._constraint = replace(self._constraint, t_max=t_max)
        self._auto_t_max = False
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
        if self._n_layers > 1:
            return self._make_layered_generator()
        if self._layer_overrides or self._start_layers != "random":
            raise ValueError("layer() and start_layers() require layers(n) with n >= 2.")
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
            n_gems=self._n_gems,
            n_boxes=self._n_boxes,
            laser_placement=placement,
            laser_span=self._laser_span,
            n_walls=self._n_walls,
            walls_style=self._walls_style,
            n_rooms_rows=self._n_rooms_rows,
            n_rooms_cols=self._n_rooms_cols,
            door_size=self._door_size,
            constraint=self._constraint,
        )

    def _make_layered_generator(self) -> LayeredGenerator:
        if self._layout_explicit and (self._starts, self._exits) != ("random", "random"):
            raise ValueError("layers() only supports the random layout.")
        if self._laser_placement not in ("auto", "free"):
            raise ValueError("layers() only supports the 'free' laser placement.")
        if self._n_rooms_rows > 0:
            raise ValueError("layers() does not support rooms().")
        if any(not 0 <= k < self._n_layers for k in self._layer_overrides):
            raise ValueError(f"layer() overrides a layer outside [0, {self._n_layers}): {sorted(self._layer_overrides)}")
        constraint = self._constraint
        if self._auto_t_max:
            constraint = replace(constraint, t_max=self._width * self._height * self._n_layers // 2)
        configs = []
        for k in range(self._n_layers):
            overrides = self._layer_overrides.get(k, {})
            n_walls = overrides.get("walls", self._n_walls)
            configs.append(
                LayerConfig(
                    n_walls=self._width * self._height // 10 if n_walls == "auto" else int(n_walls),
                    walls_style=self._walls_style,
                    n_lasers=int(overrides.get("lasers", self._resolve_n_lasers("free"))),
                    laser_span=self._laser_span,
                    n_gems=int(overrides.get("gems", self._n_gems)),
                    n_boxes=int(overrides.get("boxes", self._n_boxes)),
                    actuators=tuple(overrides.get("actuators", self._actuators)),
                )
            )
        return LayeredGenerator(
            width=self._width,
            height=self._height,
            n_agents=self._n_agents,
            n_layers=self._n_layers,
            layers=configs,
            start_layers=self._start_layers,
            max_changes=self._max_layer_changes,
            extra_exits=self._extra_exits,
            cross_layer_actuators=self._cross_layer_actuators,
            restricted_lifts=self._restricted_lifts,
            constraint=constraint,
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
