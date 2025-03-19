from abc import ABC, abstractmethod
from dataclasses import dataclass
from typing import Optional, Sequence

import numpy as np
import numpy.typing as npt

from lle import World, tiles

from .utils import get_lasers_of


@dataclass
class ExtraGenerator(ABC):
    """
    `ExtraGenerator`s are used to generate additional information of the environment aside from the observation itself.

    An example of extras is whether the subgoals of the environment have already been reached. With `n` subgoals, there would be `n` boolean extras.
    """

    size: int
    meanings: list[str]

    def __init__(self, extras_size: int, meanings: list[str]):
        self.size = extras_size
        self.meanings = meanings

    @abstractmethod
    def compute(self) -> npt.NDArray[np.float32]:
        """Compute the extras"""

    def reset(self):
        """Reset the generator state"""


@dataclass
class NoExtras(ExtraGenerator):
    def __init__(self, n_agents: int):
        super().__init__(0, [])
        self.extras = np.zeros((n_agents, 0), dtype=np.float32)

    def compute(self):
        return self.extras


@dataclass
class MultiGenerator(ExtraGenerator):
    def __init__(self, *generators: ExtraGenerator):
        size = 0
        meanings = []
        for generator in generators:
            size += generator.size
            meanings.extend(generator.meanings)
        super().__init__(size, meanings)
        self.generators = list(generators)

    def add(self, *generators: ExtraGenerator):
        for generator in generators:
            self.generators.append(generator)
            self.size += generator.size
            self.meanings.extend(generator.meanings)

    def compute(self):
        extras = []
        for generator in self.generators:
            extras.append(generator.compute())
        return np.concatenate(extras, axis=1)

    def reset(self):
        for generator in self.generators:
            generator.reset()


@dataclass
class LaserSubgoal(ExtraGenerator):
    """
    Generates extras that indicate whether the agents have reached each laser, which are subgoals of the environment.
    """

    def __init__(self, world: World, sources: Optional[Sequence[tiles.LaserSource]] = None):
        """
        Args:
            world: The world
            sources: The laser sources to consider. If None, all laser sources in the world are considered.
        """
        self.world = world
        if sources is None:
            sources = world.laser_sources
        super().__init__(len(sources), [f"Source {source.laser_id} at {source.pos}" for source in sources])
        self.pos_to_reward = [set(laser.pos for laser in get_lasers_of(world, source)) for source in sources]
        self.agents_pos_reached = np.full((world.n_agents, len(sources)), False, dtype=np.bool)

    def compute(self):
        for agent, agent_pos in enumerate(self.world.agents_positions):
            for source, pos_to_reward in enumerate(self.pos_to_reward):
                if agent_pos in pos_to_reward:
                    self.agents_pos_reached[agent, source] = True
        return self.agents_pos_reached.astype(np.float32)

    def reset(self):
        self.agents_pos_reached.fill(False)
