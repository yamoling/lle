from abc import ABC, abstractmethod
from enum import IntEnum
import numpy as np
import cv2

# pylint: disable=no-name-in-module
from lle import World


class ObservationType(IntEnum):
    """The different observation types for the World"""

    RELATIVE_POSITIONS = 0
    RGB_IMAGE = 1
    LAYERED = 2
    FLATTENED = 3

    @staticmethod
    def from_str(s: str) -> "ObservationType":
        """Convert a string to an ObservationType"""
        s = s.upper()
        for enum in ObservationType:
            if enum.name == s:
                return enum
        raise ValueError(f"'{s}' does not match any enum name from ObservationType")

    def get_observation_generator(self, world: World) -> "ObservationGenerator":
        """Get the observation generator for the observation type"""
        match self:
            case ObservationType.RELATIVE_POSITIONS:
                return RelativePosition(world)
            case ObservationType.RGB_IMAGE:
                return RGBImage(world)
            case ObservationType.LAYERED:
                return Layered(world)
            case ObservationType.FLATTENED:
                return FlattenedLayered(world)
            case other:
                raise ValueError(f"Unknown observation type: {other}")


class ObservationGenerator(ABC):
    def __init__(self, world: World) -> None:
        super().__init__()
        self._world = world

    @abstractmethod
    def observe(self) -> np.ndarray[np.float32]:
        """Observe the world and return an observation for each agent."""

    @property
    @abstractmethod
    def obs_type(self) -> ObservationType:
        """The observation type linked to the observation generator"""

    @property
    @abstractmethod
    def shape(self) -> tuple[int, ...]:
        """The observation shape of each individual agent."""

    def set_world(self, new_world: World):
        """Change the world to observe"""
        self._world = new_world


class RelativePosition(ObservationGenerator):
    def __init__(self, world) -> None:
        super().__init__(world)
        self.n_gems = world.n_gems
        self.dimensions = np.array([world.height, world.width])

    def observe(self):
        positions = np.tile((self._world.agent_positions / self.dimensions).flatten(), (self._world.n_agents, 1))
        gems_collected = np.tile(
            np.array([not gem.is_collected for _, gem in self._world.gems], dtype=np.float32), (self._world.n_agents, 1)
        )
        return np.concatenate([positions, gems_collected], axis=1).astype(np.float32)

    @property
    def obs_type(self) -> ObservationType:
        return ObservationType.RELATIVE_POSITIONS

    @property
    def shape(self):
        return (self._world.n_agents * 2 + self.n_gems,)


class RGBImage(ObservationGenerator):
    def observe(self):
        obs = self._world.get_image()
        obs = cv2.resize(obs, (120, 160))
        obs = obs.transpose(2, 1, 0)
        return np.tile(obs, (self._world.n_agents, 1, 1, 1))

    @property
    def obs_type(self) -> ObservationType:
        return ObservationType.RGB_IMAGE

    @property
    def shape(self):
        return (3, 160, 120)


class Layered(ObservationGenerator):
    """
    Layered observation of the map (walls, lasers, ...).

    Example with 4 agents:
        - Layer 0:  1 at agent 0 location
        - Layer 1:  1 at agent 1 location
        - Layer 2:  1 at agent 2 location
        - Layer 3:  1 at agent 3 location
        - Layer 4:  1 at wall locations
        - Layer 5: -1 at laser 0 sources and 1 at laser 0 beams
        - Layer 6: -1 at laser 1 sources and 1 at laser 1 beams
        - Layer 7: -1 at laser 2 sources and 1 at laser 2 beams
        - Layer 8: -1 at laser 3 sources and 1 at laser 3 beams
        - Layer 9:  1 at gem locations
        - Layer 10: 1 at end tile locations
    """

    def __init__(self, world) -> None:
        super().__init__(world)
        self.width = world.width
        self.height = world.height
        self.n_agents = world.n_agents
        self._shape = (world.n_agents * 2 + 3, world.height, world.width)
        self.A0 = 0
        self.WALL = self.A0 + world.n_agents
        self.LASER_0 = self.WALL + 1
        self.GEM = self.LASER_0 + world.n_agents
        self.EXIT = self.GEM + 1

        self.static_obs = self._setup()

    def _setup(self):
        """Initial setup with static data (walls, gems, exits)"""
        obs = np.zeros(self._shape, dtype=np.float32)
        for i, j in self._world.exit_pos:
            obs[self.EXIT, i, j] = 1.0

        for i, j in self._world.wall_pos:
            obs[self.WALL, i, j] = 1.0

        for (i, j), source in self._world.laser_sources:
            obs[self.LASER_0 + source.agent_id, i, j] = -1.0
            obs[self.WALL, i, j] = 1.0

        return obs

    def observe(self):
        obs = np.copy(self.static_obs)
        for (i, j), laser in self._world.lasers:
            if laser.is_on:
                obs[self.LASER_0 + laser.agent_id, i, j] = 1.0
        for (i, j), gem in self._world.gems:
            if not gem.is_collected:
                obs[self.GEM, i, j] = 1.0
        for i, (y, x) in enumerate(self._world.agent_positions):
            obs[self.A0 + i, y, x] = 1.0
        return np.tile(obs, (self.n_agents, 1, 1, 1))

    @property
    def shape(self):
        return self._shape

    @property
    def obs_type(self) -> ObservationType:
        return ObservationType.LAYERED


class FlattenedLayered(ObservationGenerator):
    def __init__(self, world) -> None:
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

    def set_world(self, new_world: World):
        self.layered.set_world(new_world)
        return super().set_world(new_world)
