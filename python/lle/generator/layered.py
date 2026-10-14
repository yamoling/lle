"""Multi-layer world generation: layers linked by lifts and buttons.

A level is generated in three stages (see `docs/superpowers/specs/2026-10-12-layer-generator-design.md`):

1. **Skeleton.** Random journeys over the layers decide the lifts, their landing cells, the buttons,
   the starts and the exits of every layer at once, since a lift constrains two layers.
2. **Layers.** Each layer is filled with walls, lasers, gems and boxes by the single-layer
   placement, then checked by a BFS: every visit of the layer can reach where its journey goes next,
   and every lift has a presser who can reach one of its buttons.
3. **Level.** The layers are stacked and the whole-level constraint is checked by the solver. The
   shortest plan must also visit every layer.
"""

from __future__ import annotations

import random
from collections import deque
from dataclasses import dataclass, field
from typing import Literal, Sequence

from ..characterization.world_characterization import WorldCharacterizer
from ..types import Position
from ..world import World
from .candidates import CandidateLayout
from .generator import WorldGenerator
from .placements import LayoutRetry, PlacementCtx, place_lasers, place_on_free_cells, place_walls
from .world_builder import WorldBuilder, build_layers
from .world_filter import Constraint

# Tries before giving up on a skeleton (journeys visiting every layer, free cells for lifts).
SKELETON_TRIES = 50
# Fills of one layer before drawing a new skeleton.
LAYER_TRIES = 20


@dataclass
class LayerConfig:
    """What the single-layer placement adds on one layer."""

    n_walls: int
    walls_style: Literal["individual", "shapes"]
    n_lasers: int
    laser_span: int | Literal["any", "across"]
    n_gems: int
    n_boxes: int


@dataclass
class Lift:
    layer: int
    pos: Position
    up: bool
    group: int
    riders: set[int] = field(default_factory=set)
    colour: int | None = None

    @property
    def landing(self):
        return self.layer + (1 if self.up else -1)


@dataclass
class Button:
    layer: int
    pos: Position
    group: int
    colour: int | None = None


@dataclass
class Visit:
    """One agent on one layer, from the cell it begins at to the cell its journey goes to next."""

    agent: int
    layer: int
    begin: Position
    target: Position


@dataclass
class Skeleton:
    journeys: list[list[int]]
    starts: list[Position]
    exits: list[tuple[int, Position]]
    lifts: dict[tuple[int, bool], Lift]
    buttons: list[Button]
    visits: list[Visit]

    def reserved(self, layer: int):
        """Cells of `layer` the skeleton uses, landing cells included."""
        cells = {pos for agent, pos in enumerate(self.starts) if self.journeys[agent][0] == layer}
        cells |= {pos for k, pos in self.exits if k == layer}
        cells |= {lift.pos for lift in self.lifts.values() if layer in (lift.layer, lift.landing)}
        cells |= {button.pos for button in self.buttons if button.layer == layer}
        return cells

    def visitors(self, layer: int):
        return {visit.agent for visit in self.visits if visit.layer == layer}


def draw_journeys(start_layers: Sequence[int], n_layers: int, max_changes: int, rng: random.Random):
    """One random walk over the layers per agent; redrawn until every layer is visited."""
    for _ in range(SKELETON_TRIES):
        journeys = []
        for layer in start_layers:
            journey = [layer]
            for _ in range(rng.randint(0, max_changes)):
                journey.append(rng.choice([k for k in (journey[-1] - 1, journey[-1] + 1) if 0 <= k < n_layers]))
            journeys.append(journey)
        if {k for journey in journeys for k in journey} == set(range(n_layers)):
            return journeys
    raise LayoutRetry()


class LayeredGenerator(WorldGenerator):
    """Generate worlds with several layers. See the module documentation for the stages."""

    def __init__(
        self,
        *,
        width: int,
        height: int,
        n_agents: int,
        n_layers: int,
        layers: Sequence[LayerConfig],
        start_layers: Sequence[int] | Literal["random"] = "random",
        max_changes: int | None = None,
        extra_exits: int = 0,
        cross_layer_buttons: bool = False,
        restricted_lifts: float = 0.0,
        constraint: Constraint | None = None,
    ):
        if n_layers < 2:
            raise ValueError(f"A layered world needs at least 2 layers. Got {n_layers}")
        if len(layers) != n_layers:
            raise ValueError(f"Expected one LayerConfig per layer ({n_layers}), got {len(layers)}")
        if start_layers != "random":
            if len(start_layers) != n_agents:
                raise ValueError(f"start_layers needs one layer per agent ({n_agents}). Got {list(start_layers)}")
            if any(not 0 <= k < n_layers for k in start_layers):
                raise ValueError(f"start_layers must be in [0, {n_layers}). Got {list(start_layers)}")
        if extra_exits < 0:
            raise ValueError(f"extra_exits must be >= 0. Got {extra_exits}")
        if not 0.0 <= restricted_lifts <= 1.0:
            raise ValueError(f"restricted_lifts is a probability. Got {restricted_lifts}")
        for k, cfg in enumerate(layers):
            if cfg.n_lasers > n_agents:
                raise ValueError(f"Layer {k}: lasers must be <= agents (one source per colour). Got {cfg.n_lasers}")
        self.width, self.height = width, height
        self.agents = n_agents
        self.n_layers = n_layers
        self.layer_configs = list(layers)
        self.start_layers = start_layers
        self.max_changes = n_layers if max_changes is None else max_changes
        self.extra_exits = extra_exits
        self.cross_layer_buttons = cross_layer_buttons
        self.restricted_lifts = restricted_lifts
        self.constraint = constraint
        self._rng = random.Random()

    # ------------------------------------------------------------------
    # Skeleton
    # ------------------------------------------------------------------
    def _free_cell(self, used: list[set[Position]], *layers: int):
        """A random cell free on every layer of `layers`, then reserved on all of them."""
        free = [(i, j) for i in range(self.height) for j in range(self.width) if all((i, j) not in used[k] for k in layers)]
        if not free:
            raise LayoutRetry()
        pos = self._rng.choice(free)
        for k in layers:
            used[k].add(pos)
        return pos

    def _make_skeleton(self):
        rng = self._rng
        start_layers = [rng.randrange(self.n_layers) for _ in range(self.agents)] if self.start_layers == "random" else self.start_layers
        journeys = draw_journeys(start_layers, self.n_layers, self.max_changes, rng)
        used: list[set[Position]] = [set() for _ in range(self.n_layers)]

        # One lift per layer transition, shared by every agent making it, in its own group.
        lifts: dict[tuple[int, bool], Lift] = {}
        for agent, journey in enumerate(journeys):
            for k, nxt in zip(journey, journey[1:]):
                key = (k, nxt > k)
                if key not in lifts:
                    lifts[key] = Lift(k, self._free_cell(used, k, nxt), nxt > k, group=len(lifts))
                lifts[key].riders.add(agent)

        starts = [self._free_cell(used, journey[0]) for journey in journeys]
        exits = [(journey[-1], self._free_cell(used, journey[-1])) for journey in journeys]
        for _ in range(self.extra_exits):
            k = rng.randrange(self.n_layers)
            exits.append((k, self._free_cell(used, k)))

        visits = []
        for agent, journey in enumerate(journeys):
            begin = starts[agent]
            for step, k in enumerate(journey):
                if step + 1 < len(journey):
                    lift = lifts[(k, journey[step + 1] > k)]
                    visits.append(Visit(agent, k, begin, lift.pos))
                    begin = lift.pos
                else:
                    visits.append(Visit(agent, k, begin, exits[agent][1]))

        buttons = []
        for lift in lifts.values():
            layers = [lift.layer, lift.landing] if self.cross_layer_buttons else [lift.layer]
            for k in layers:
                buttons.append(Button(k, self._free_cell(used, k), lift.group))
        skeleton = Skeleton(journeys, starts, exits, lifts, buttons, visits)
        self._restrict_colours(skeleton)
        return skeleton

    def _restrict_colours(self, skeleton: Skeleton):
        """Restrict some lifts to their single rider, and their buttons to a possible presser."""
        for lift in skeleton.lifts.values():
            if len(lift.riders) != 1 or self._rng.random() >= self.restricted_lifts:
                continue
            (lift.colour,) = lift.riders
            for button in skeleton.buttons:
                if button.group == lift.group:
                    pressers = sorted(skeleton.visitors(button.layer) - lift.riders)
                    if pressers:
                        button.colour = self._rng.choice(pressers)

    # ------------------------------------------------------------------
    # Layers
    # ------------------------------------------------------------------
    def _fill_layer(self, k: int, skeleton: Skeleton):
        """Walls, lasers, gems and boxes around the skeleton cells of layer `k`."""
        cfg = self.layer_configs[k]
        rng = self._rng
        reserved = skeleton.reserved(k)
        # Beams never cover a reserved cell, so they never cross a lift, a landing or a button.
        lasers, reserved = place_lasers(
            cfg.n_lasers, "free", cfg.laser_span, self.agents, self.height, self.width, rng, reserved, PlacementCtx()
        )
        walls = place_walls(cfg.n_walls, cfg.walls_style, reserved, self.height, self.width, rng)
        gems, reserved = place_on_free_cells(cfg.n_gems, reserved | set(walls), self.height, self.width, rng)
        boxes, _ = place_on_free_cells(cfg.n_boxes, reserved, self.height, self.width, rng)
        layout = CandidateLayout(
            self.height,
            self.width,
            agents=[pos for agent, pos in enumerate(skeleton.starts) if skeleton.journeys[agent][0] == k],
            exits=[pos for layer, pos in skeleton.exits if layer == k],
            gems=gems,
            walls=walls,
            lasers=lasers,
            boxes=boxes,
        )
        if not layout.is_geometry_valid() or not self._is_reachable(k, layout, skeleton):
            raise LayoutRetry()
        return layout

    def _is_reachable(self, k: int, layout: CandidateLayout, skeleton: Skeleton):
        """BFS check of layer `k`: walls and laser sources block; beams and boxes do not, since a beam
        may be blocked and a box pushed. Exits are absorbing, so only a visit's own target is entered."""
        blocked = set(layout.walls) | {pos for _, pos, _ in layout.lasers}
        exits = set(layout.exits)

        def reachable(begin: Position, target: Position):
            seen = {begin}
            queue = deque([begin])
            while queue:
                i, j = queue.popleft()
                if (i, j) == target:
                    return True
                for nxt in ((i - 1, j), (i + 1, j), (i, j - 1), (i, j + 1)):
                    if (
                        0 <= nxt[0] < self.height
                        and 0 <= nxt[1] < self.width
                        and nxt not in seen
                        and nxt not in blocked
                        and (nxt not in exits or nxt == target)
                    ):
                        seen.add(nxt)
                        queue.append(nxt)
            return False

        visits = [visit for visit in skeleton.visits if visit.layer == k]
        if not all(reachable(visit.begin, visit.target) for visit in visits):
            return False
        # Every lift leaving or reaching this layer through one of its buttons needs a presser here
        # (an agent not riding it), or among several riders who can take turns.
        for lift in skeleton.lifts.values():
            buttons = [b for b in skeleton.buttons if b.group == lift.group and b.layer == k]
            if not buttons:
                continue
            if not any(
                (visit.agent not in lift.riders or len(lift.riders) > 1)
                and any(b.colour in (None, visit.agent) and reachable(visit.begin, b.pos) for b in buttons)
                for visit in visits
            ):
                return False
        return True

    # ------------------------------------------------------------------
    # Level
    # ------------------------------------------------------------------
    def _build_world(self, skeleton: Skeleton, layouts: list[CandidateLayout]) -> World:
        builders = []
        for k, layout in enumerate(layouts):
            b = WorldBuilder(self.width, self.height)
            for agent, pos in enumerate(skeleton.starts):
                if skeleton.journeys[agent][0] == k:
                    b.add_agent(agent, pos)
            for pos in layout.exits:
                b.add_exit(pos)
            for lift in skeleton.lifts.values():
                if lift.layer == k:
                    b.add_lift(lift.pos, lift.up, lift.group, lift.colour)
            for button in skeleton.buttons:
                if button.layer == k:
                    b.add_button(button.pos, button.group, button.colour)
            for pos in layout.walls:
                b.add_wall(pos)
            for pos in layout.gems:
                b.add_gem(pos)
            for pos in layout.boxes:
                b.add_box(pos)
            for owner, pos, direction in layout.lasers:
                b.add_laser(owner, pos, direction)
            builders.append(b)
        return build_layers(builders)

    def _try_generate(self, seed: int | None) -> World | None:
        if seed is not None:
            self._rng.seed(seed)
        try:
            skeleton = self._make_skeleton()
            layouts = [self._fill_layer_retrying(k, skeleton) for k in range(self.n_layers)]
        except LayoutRetry:
            return None
        world = self._build_world(skeleton, layouts)
        if self._accept_world(world):
            return world
        return None

    def _fill_layer_retrying(self, k: int, skeleton: Skeleton):
        for _ in range(LAYER_TRIES):
            try:
                return self._fill_layer(k, skeleton)
            except LayoutRetry:
                continue
        raise LayoutRetry()

    def _accept_world(self, world: World):
        if self.constraint is None:
            return True
        c = WorldCharacterizer(world, self.constraint.t_max)
        try:
            if not self.constraint.is_satisfied_by(world, c):
                return False
        except BaseException as e:
            raise RuntimeError(f"Error while checking world constraints: {e}") from e
        return c.shortest_path is not None and visits_every_layer(world, c.shortest_path)


def visits_every_layer(world: World, plan):
    """Whether some agent stands on each layer at some point of `plan`."""
    world.reset()
    visited = {pos[2] for pos in world.agents_positions}
    for joint_action in plan:
        world.step(joint_action)
        visited |= {pos[2] for pos in world.agents_positions}
    world.reset()
    return visited == set(range(world.layers))
