"""Observation types and observation generators for `LLE`.

This module defines the public observation presets accepted by the Python API
and the internal generators that turn a `World` into arrays or images.
Use `ObservationType.from_str(...)` when you accept user input, and use
`get_observation_generator(...)` to build the concrete generator for a world.
"""

from abc import ABC, abstractmethod
from dataclasses import dataclass
from enum import Enum
from typing import Literal, Sequence

import numpy as np
import numpy.typing as npt

from lle.world import World, WorldState

from .types import AgentId, Position

ObservationTypeLiteral = Literal[
    "layered",
    "flattened",
    "partial3x3",
    "partial5x5",
    "partial7x7",
    "state",
    "rgb-image",
    "perspective",
    "normalized-state",
    "layered-padded-1",
    "layered-padded-2",
    "layered-padded-3",
    "layered-padded",
]


class ObservationType(str, Enum):
    """Public observation presets supported by the environment."""

    NORMALIZED_STATE = "normalized-state"
    STATE = "state"
    """The state of the world (agents' positions, alive status, gems collections) as a flat vector."""
    RGB_IMAGE = "rgb-image"
    """The rendered world as an RGB image."""
    LAYERED = "layered"
    """
    Layered observations of the map (walls, lasers, ...) as shown below. Only 2 agents are shown for the sake of clarity.

    ![Layered representation of the world](../../docs/layers.png)
    """
    FLATTENED = "flattened"
    """The layered representation flattened to one dimension."""
    PARTIAL_3x3 = "partial3x3"
    PARTIAL_5x5 = "partial5x5"
    PARTIAL_7x7 = "partial7x7"
    LAYERED_PADDED = "layered-padded"
    LAYERED_PADDED_1AGENT = "layered-padded-1"
    LAYERED_PADDED_2AGENTS = "layered-padded-2"
    LAYERED_PADDED_3AGENTS = "layered-padded-3"
    PERSPECTIVE = "perspective"

    @staticmethod
    def from_str(s: ObservationTypeLiteral | str) -> "ObservationType":
        return ObservationType(s)

    def get_observation_generator(self, world: World, padding_size: int = 0) -> "ObservationGenerator":
        match self:
            case ObservationType.NORMALIZED_STATE:
                return StateGenerator(world, normalize=True)
            case ObservationType.STATE:
                return StateGenerator(world, normalize=False)
            case ObservationType.RGB_IMAGE:
                return RGBImage(world)
            case ObservationType.LAYERED:
                return Layered(world)
            case ObservationType.FLATTENED:
                return FlattenedLayered(world)
            case ObservationType.PARTIAL_3x3:
                return PartialGenerator(world, 3)
            case ObservationType.PARTIAL_5x5:
                return PartialGenerator(world, 5)
            case ObservationType.PARTIAL_7x7:
                return PartialGenerator(world, 7)
            case ObservationType.LAYERED_PADDED:
                return LayeredPadded(world, padding_size)
            case ObservationType.LAYERED_PADDED_1AGENT:
                return LayeredPadded(world, 1)
            case ObservationType.LAYERED_PADDED_2AGENTS:
                return LayeredPadded(world, 2)
            case ObservationType.LAYERED_PADDED_3AGENTS:
                return LayeredPadded(world, 3)
            case ObservationType.PERSPECTIVE:
                return PerspectiveLayered(world)
            case other:
                raise ValueError(f"Unknown observation type: {other}")


def require_unshared_colours(world: World, generator_name: str) -> None:
    """Refuse to build a non-centred observation for a world where two agents share a colour.

    Agents of one colour are indistinguishable in such an observation: they appear in the same
    layer, and nothing in the array says which of them the observation is addressed to. Only
    `perspective`, which centres the map on the observing agent, stays unambiguous.
    """
    seen = dict[int, int]()
    for agent_num, colour in enumerate(world.agent_colours):
        if colour in seen:
            raise ValueError(
                f"{generator_name} cannot represent a world where agents share a colour: agents "
                f"{seen[colour]} and {agent_num} both have colour {colour}, so neither could tell "
                f'which of them the observation describes. Use the "perspective" observation, '
                "which centres the map on the observing agent."
            )
        seen[colour] = agent_num


@dataclass
class ObservationGenerator(ABC):
    """Base class for world-to-observation converters."""

    def __init__(self, world: World):
        super().__init__()
        self._world = world

    @abstractmethod
    def observe(self) -> npt.NDArray[np.float32]:
        """Return the observation for every agent."""

    def get_state(self) -> npt.NDArray[np.float32]:
        return self.observe()[0]

    def to_world_state(self, data: npt.NDArray[np.float32]) -> WorldState:
        """Convert observation data back into a `WorldState`.

        Generators that cannot reconstruct a world state should override this method.
        """
        raise NotImplementedError(f"This method is not implemented for {self.__class__.__name__}")

    @property
    @abstractmethod
    def obs_type(self) -> ObservationType:
        """The observation preset represented by this generator."""

    @property
    @abstractmethod
    def shape(self) -> tuple[int, ...]:
        """The shape of a single-agent observation."""

    def set_world(self, new_world: World):
        """Point the generator at another world."""
        self._world = new_world

    def reset(self) -> None:
        """Refresh any state cached across calls to `observe()` (e.g. static layers derived
        from world topology). Generators that cache such data must be told when it may have
        changed: `LLE.reset()` calls this after every episode reset (covering e.g.
        `randomize_lasers`), and any code that mutates world topology directly (e.g.
        `world.exit_pos = ...`) outside of `LLE` must call it explicitly afterwards. The
        default implementation is a no-op; generators with no such cache need not override it.
        """


class StateGenerator(ObservationGenerator):
    def __init__(self, world: World, normalize: bool):
        super().__init__(world)
        self.n_gems = world.n_gems
        self.n_agents = world.n_agents
        if normalize:
            self.dimensions = np.array([world.height, world.width] * world.n_agents)
        else:
            self.dimensions = np.array([1.0, 1.0] * world.n_agents)

    def observe(self):
        state = self._world.get_state().as_array()
        state[: self._world.n_agents * 2] = state[: self._world.n_agents * 2] / self.dimensions
        return np.tile(state, reps=(self._world.n_agents, 1))

    def to_world_state(self, data):
        data[: self._world.n_agents * 2] = data[: self._world.n_agents * 2] * self.dimensions
        return WorldState.from_array(data.tolist(), self.n_agents, self.n_gems)

    @property
    def obs_type(self) -> ObservationType:
        return ObservationType.STATE

    @property
    def shape(self):
        """The full world state: (i, j) for each agent, each gem's collection status, and each agent's alive flag."""
        return (self._world.n_agents * 3 + self.n_gems,)

    @property
    def unit_size(self) -> int:
        return 2


class RGBImage(ObservationGenerator):
    def __init__(self, world: World):
        super().__init__(world)
        self._shape = tuple(world.get_image().shape)

    def observe(self):
        obs = self._world.get_image()
        return np.tile(obs, (self._world.n_agents, 1, 1, 1)).astype(np.float32)

    @property
    def obs_type(self) -> ObservationType:
        return ObservationType.RGB_IMAGE

    @property
    def shape(self):
        return self._shape


@dataclass
class LayeredPadded(ObservationGenerator):
    """Layered observations with an optional agent padding budget."""

    def __init__(self, world: World, padding_size: int, *, guard_shared_colours: bool = True):
        super().__init__(world)
        if guard_shared_colours:
            require_unshared_colours(world, type(self).__name__)
        self.width = world.width
        self.height = world.height
        self.n_agents = world.n_agents + padding_size
        # One agent layer and one laser layer per colour. Sizing the bands by the colour space
        # rather than by `n_agents` is what keeps a laser colour above `n_agents` from aliasing
        # into the WALL band.
        self.n_colours = world.n_colours + padding_size
        self.A0 = 0
        self.LASER_0 = self.A0 + self.n_colours
        self.WALL = self.LASER_0 + self.n_colours
        self.VOID = self.WALL + 1
        self.GEM = self.VOID + 1
        self.EXIT = self.GEM + 1
        self._shape = (self.EXIT + 1, world.height, world.width)
        self.ordered_gem_pos = sorted(gem.pos for gem in world.gems)

        self.static_obs = self._setup()

    def _setup(self):
        """Compute the layers that are constant for an episode: walls and voids never change;
        exit positions and laser source positions/colours only change (if at all) via
        `world.reset()`-adjacent events (e.g. `randomize_lasers`) or explicit topology edits
        (e.g. `world.exit_pos = ...`), both of which call `reset()` on this generator — see
        `ObservationGenerator.reset`. Recomputed there instead of on every `observe()` call.
        """
        obs = np.zeros(self._shape, dtype=np.float32)
        for i, j in self._world.wall_pos:
            obs[self.WALL, i, j] = 1.0

        for i, j in self._world.void_pos:
            obs[self.VOID, i, j] = 1.0

        for i, j in self._world.exit_pos:
            obs[self.EXIT, i, j] = 1.0

        for source in self._world.laser_sources:
            i, j = source.pos
            obs[self.LASER_0 + source.colour, i, j] = -1.0

        return obs

    def reset(self) -> None:
        self.static_obs = self._setup()

    def to_world_state(self, data: npt.NDArray[np.float32]) -> WorldState:
        """Reconstruct a world state from a layered observation.

        This assumes that all agents are alive.
        """
        _, i, j = np.nonzero(data[self.A0 : self.A0 + self.n_colours])
        agents_positions = [(int(i[n]), int(j[n])) for n in range(self._world.n_agents)]
        gems_collected = []
        for i, j in self.ordered_gem_pos:
            gems_collected.append(bool(data[self.GEM, i, j] == 0.0))
        return WorldState(agents_positions, gems_collected)

    def observe(self):
        obs = np.copy(self.static_obs)
        for laser in self._world.lasers:
            i, j = laser.pos
            if laser.is_on:
                obs[self.LASER_0 + laser.colour, i, j] = 1.0
        for gem in self._world.gems:
            i, j = gem.pos
            if not gem.is_collected:
                obs[self.GEM, i, j] = 1.0
        # Every agent is stamped into its colour's layer. Two agents never share a cell, so
        # same-colour agents cannot overwrite each other.
        for colour, (y, x) in zip(self._world.agent_colours, self._world.agents_positions):
            obs[self.A0 + colour, y, x] = 1.0
        return np.tile(obs, (self._world.n_agents, 1, 1, 1))

    @property
    def shape(self):
        return self._shape

    @property
    def obs_type(self) -> ObservationType:
        return ObservationType.LAYERED


class Layered(LayeredPadded):
    def __init__(self, world: World, *, guard_shared_colours: bool = True):
        super().__init__(world, padding_size=0, guard_shared_colours=guard_shared_colours)


class FlattenedLayered(ObservationGenerator):
    def __init__(self, world: World):
        super().__init__(world)
        self.layered = Layered(world)
        size = 1
        for s in self.layered.shape:
            size = size * s
        self._shape = (size,)

    def observe(self):
        obs = self.layered.observe()
        return obs.reshape(self._world.n_agents, -1)

    @property
    def obs_type(self) -> ObservationType:
        return ObservationType.FLATTENED

    @property
    def shape(self):
        return self._shape

    @property
    def unit_size(self) -> int:
        return 0

    def set_world(self, new_world: World):
        self.layered.set_world(new_world)
        return super().set_world(new_world)


class PartialGenerator(ObservationGenerator):
    def __init__(self, world: World, square_size: int):
        super().__init__(world)
        assert square_size % 2 == 1, "Can only use odd numbers for the square size"
        require_unshared_colours(world, type(self).__name__)
        self.size = square_size
        # One layer per colour for the agents, one per colour for the lasers, walls, gems, exits
        self.n_colours = world.n_colours
        self._shape = (self.n_colours * 2 + 3, self.size, self.size)
        self._center = self.size // 2
        self.WALL = self.n_colours
        self.LASER_0 = self.WALL + 1
        self.GEM = self.LASER_0 + self.n_colours
        self.EXIT = self.GEM + 1

    @property
    def shape(self) -> tuple[int, int, int]:
        return self._shape

    @property
    def obs_type(self) -> ObservationType:
        return ObservationType.PARTIAL_3x3

    def encode_layer(self, layer: npt.NDArray[np.float32], origin: Position, positions: Sequence[Position], fill_value: float = 1.0):
        if len(positions) == 0:
            return
        for i, j in positions:
            i, j = i - origin[0] + self._center, j - origin[1] + self._center
            if 0 <= i < self.size and 0 <= j < self.size:
                layer[i, j] = fill_value

    def observe(self) -> npt.NDArray[np.float32]:
        obs = np.zeros((self._world.n_agents, *self._shape), dtype=np.float32)
        colours = self._world.agent_colours
        for a, agent_pos in enumerate(self._world.agents_positions):
            # Agents positions, grouped by colour
            for colour, other_pos in zip(colours, self._world.agents_positions):
                self.encode_layer(obs[a, colour], agent_pos, [other_pos])
            # Gems
            self.encode_layer(obs[a, self.GEM], agent_pos, [gem.pos for gem in self._world.gems if not gem.is_collected])
            # Exits
            self.encode_layer(obs[a, self.EXIT], agent_pos, [exit_pos for exit_pos in self._world.exit_pos])
            # Walls
            self.encode_layer(obs[a, self.WALL], agent_pos, [wall_pos for wall_pos in self._world.wall_pos])
            # Lasers
            laser_positions = self._get_lasers_positions()
            for colour, positions in laser_positions.items():
                self.encode_layer(obs[a, self.LASER_0 + colour], agent_pos, positions)
            # Laser sources
            for source in self._world.laser_sources:
                self.encode_layer(obs[a, self.LASER_0 + source.colour], agent_pos, [source.pos], fill_value=-1.0)
        return obs

    def _get_lasers_positions(self) -> dict[AgentId, list[Position]]:
        """Beam tiles that are currently on, grouped by colour."""
        laser_positions = dict[AgentId, list[Position]]()
        for laser in self._world.lasers:
            if laser.is_on:
                lasers = laser_positions.get(laser.colour, [])
                lasers.append(laser.pos)
                laser_positions[laser.colour] = lasers
        return laser_positions


class PerspectiveLayered(ObservationGenerator):
    """Full-observability layered observation, centred on the observing agent.

    Unlike `Layered`, this stays unambiguous when several agents share a colour, because the
    observing agent is identified by its position rather than by a layer index:

    - the canvas is `(2·height − 1, 2·width − 1)` and the observing agent sits at its exact
      centre, so the whole map fits wherever the agent stands;
    - cells outside the map are marked in the `WALL` layer, so an agent near an edge sees the
      outside as impassable rather than as empty space;
    - there is one agent layer and one laser layer per colour, and every agent is stamped into
      its colour's layer, exactly as lasers already were;
    - the observing agent's colour is transposed with colour 0, so an agent always sees itself
      as colour 0 regardless of the colour it was assigned.
    """

    def __init__(self, world: World):
        super().__init__(world)
        self.height = world.height
        self.width = world.width
        self.n_colours = world.n_colours
        self.A0 = 0
        self.LASER_0 = self.A0 + self.n_colours
        self.WALL = self.LASER_0 + self.n_colours
        self.VOID = self.WALL + 1
        self.GEM = self.VOID + 1
        self.EXIT = self.GEM + 1
        self._shape = (self.EXIT + 1, 2 * world.height - 1, 2 * world.width - 1)
        # The map-sized view this one shifts and permutes. It is built unguarded on purpose:
        # sharing a colour is exactly the case this generator exists to handle.
        self._layered = Layered(world, guard_shared_colours=False)

    @property
    def shape(self):
        return self._shape

    @property
    def obs_type(self) -> ObservationType:
        return ObservationType.PERSPECTIVE

    def set_world(self, new_world: World):
        self._layered.set_world(new_world)
        return super().set_world(new_world)

    def reset(self) -> None:
        self._layered.reset()

    def observe(self):
        # The map-sized layers, then one shifted, colour-canonicalised copy per agent.
        world_view = self._layered.observe()[0]
        obs = np.zeros((self._world.n_agents, *self._shape), dtype=np.float32)
        # Everything outside the map is a wall; the map window is cleared per agent below.
        obs[:, self.WALL] = 1.0
        colours = self._world.agent_colours
        for agent_num, (i, j) in enumerate(self._world.agents_positions):
            di, dj = self.height - 1 - i, self.width - 1 - j
            window = self._permute_colours(world_view, colours[agent_num])
            obs[agent_num, :, di : di + self.height, dj : dj + self.width] = window
        return obs

    def _permute_colours(self, world_view: npt.NDArray[np.float32], colour: int):
        """Swap `colour` into slot 0 of both the agent band and the laser band."""
        if colour == 0:
            return world_view
        permuted = np.copy(world_view)
        for band in (self.A0, self.LASER_0):
            permuted[[band, band + colour]] = permuted[[band + colour, band]]
        return permuted
