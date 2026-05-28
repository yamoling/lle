"""Observation types and observation generators for `LLE`.

This module defines the public observation presets accepted by the Python API
and the internal generators that turn a `World` into arrays or images.
Use `ObservationType.from_str(...)` when you accept user input, and use
`get_observation_generator(...)` to build the concrete generator for a world.
"""

from abc import ABC, abstractmethod
from dataclasses import dataclass
from enum import Enum
from typing import Literal

import numpy as np
import numpy.typing as npt

from .rust_observations import Layered as RustLayered
from .rust_observations import PartialGenerator as RustPartialGenerator
from .rust_observations import StateGenerator as RustStateGenerator
from .world import World, WorldState

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
    AGENT0_PERSPECTIVE_LAYERED = "perspective"

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
            case ObservationType.AGENT0_PERSPECTIVE_LAYERED:
                return AgentZeroPerspective(world)
            case other:
                raise ValueError(f"Unknown observation type: {other}")


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


class StateGenerator(ObservationGenerator):
    def __init__(self, world: World, normalize: bool):
        super().__init__(world)
        self.n_gems = world.n_gems
        self.n_agents = world.n_agents
        self.normalize = normalize
        self._rust_generator = RustStateGenerator(world, normalize)
        if normalize:
            self.dimensions = np.array([world.height, world.width] * world.n_agents)
        else:
            self.dimensions = np.array([1.0, 1.0] * world.n_agents)

    def observe(self):
        return self._rust_generator.observe(self._world)

    def to_world_state(self, data):
        data[: self._world.n_agents * 2] = data[: self._world.n_agents * 2] * self.dimensions
        return WorldState.from_array(data.tolist(), self.n_agents, self.n_gems)

    @property
    def obs_type(self) -> ObservationType:
        return ObservationType.STATE

    @property
    def shape(self):
        """Each agent sees its position, alive flag, and gem status."""
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

    def __init__(self, world: World, padding_size: int):
        super().__init__(world)
        self.width = world.width
        self.height = world.height
        self.n_agents = world.n_agents + padding_size
        self._rust_generator = RustLayered(world, padding_size)
        if len(world.laser_sources) > 0:
            self.highest_laser_agent_id = max(source.agent_id for source in world.laser_sources)
        else:
            self.highest_laser_agent_id = 0
        self.A0 = 0
        self.WALL = self.A0 + self.n_agents
        self.LASER_0 = self.WALL + 1
        self.VOID = self.LASER_0 + self.highest_laser_agent_id + 1
        self.GEM = self.VOID + 1
        self.EXIT = self.GEM + 1
        self._shape = (self.EXIT + 1, world.height, world.width)
        self.ordered_gem_pos = sorted(gem.pos for gem in world.gems)

    def to_world_state(self, data: npt.NDArray[np.float32]) -> WorldState:
        """Reconstruct a world state from a layered observation.

        This assumes that all agents are alive.
        """
        _, i, j = np.nonzero(data[self.A0 : self.A0 + self.n_agents])
        agents_positions = [(int(i[n]), int(j[n])) for n in range(self.n_agents)]
        gems_collected = []
        for i, j in self.ordered_gem_pos:
            gems_collected.append(bool(data[self.GEM, i, j] == 0.0))
        return WorldState(agents_positions, gems_collected)

    def observe(self):
        return self._rust_generator.observe(self._world)

    @property
    def shape(self):
        return self._shape

    @property
    def obs_type(self) -> ObservationType:
        return ObservationType.LAYERED


class Layered(LayeredPadded):
    def __init__(self, world: World):
        super().__init__(world, padding_size=0)


class FlattenedLayered(ObservationGenerator):
    def __init__(self, world):
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


def distance(agent_pos: tuple[int, int], other_pos: tuple[int, int]) -> int:
    return abs(agent_pos[0] - other_pos[0]) + abs(agent_pos[1] - other_pos[1])


class PartialGenerator(ObservationGenerator):
    def __init__(self, world: World, square_size: int):
        super().__init__(world)
        assert square_size % 2 == 1, "Can only use odd numbers for the square size"
        self.size = square_size
        self._rust_generator = RustPartialGenerator(world, square_size)
        # Each agent, each laser, walls, gems, exits
        self._shape = (world.n_agents + world.n_agents + 3, self.size, self.size)
        self._center = self.size // 2
        self.WALL = world.n_agents
        self.LASER_0 = self.WALL + 1
        self.GEM = self.LASER_0 + world.n_agents
        self.EXIT = self.GEM + 1

    @property
    def shape(self) -> tuple[int, int, int]:
        return self._shape

    @property
    def obs_type(self) -> ObservationType:
        return ObservationType.PARTIAL_3x3

    def observe(self) -> npt.NDArray[np.float32]:
        return self._rust_generator.observe(self._world)


class AgentZeroPerspective(Layered):
    def __init__(self, world: World):
        super().__init__(world)

    def observe(self):
        obs = super().observe()
        # Agent 0 does not have to change
        for agent_num in range(1, self.n_agents):
            agent_obs = obs[agent_num]
            # Swap agent 0 and agent_num
            agent_zero_layer = np.copy(agent_obs[self.A0])
            agent_obs[self.A0] = agent_obs[self.A0 + agent_num]
            agent_obs[self.A0 + agent_num] = agent_zero_layer

            # Swap laser 0 and laser_num
            laser_zero_layer = np.copy(agent_obs[self.LASER_0])
            agent_obs[self.LASER_0] = agent_obs[self.LASER_0 + agent_num]
            agent_obs[self.LASER_0 + agent_num] = laser_zero_layer

        return obs
