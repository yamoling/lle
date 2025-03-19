from abc import ABC, abstractmethod
from dataclasses import dataclass
from typing import Iterable
from functools import cached_property

import numpy as np
import numpy.typing as npt
from marlenv import DiscreteSpace

from lle import EventType, World, WorldEvent
from lle import tiles
from lle.types import Position

from .utils import get_lasers_of

REWARD_GEM = 1.0
REWARD_EXIT = 1.0
REWARD_DONE = 1.0
REWARD_DEATH = -1.0


@dataclass
class RewardStrategy(ABC):
    objectives: list[str]
    n_agents: int
    reward_space: DiscreteSpace
    n_arrived: int
    n_deads: int

    def __init__(self, n_agents: int, objectives: list[str]):
        self.objectives = objectives
        self.reward_space = DiscreteSpace(len(objectives), objectives)
        self.n_agents = n_agents
        self.n_arrived = 0
        self.n_deads = 0

    @cached_property
    def n_objectives(self) -> int:
        return self.reward_space.shape[0]

    def reset(self):
        self.n_arrived = 0
        self.n_deads = 0

    @abstractmethod
    def compute_reward(self, events: list[WorldEvent]) -> npt.NDArray[np.float32]:
        """Compute the reward for the given events."""


@dataclass
class SingleObjective(RewardStrategy):
    def __init__(self, n_agents: int):
        super().__init__(n_agents, ["reward"])

    def compute_reward(self, events: list[WorldEvent]):
        reward = 0.0
        death_reward = 0.0
        for event in events:
            match event.event_type:
                case EventType.AGENT_DIED:
                    reward += REWARD_DEATH
                    self.n_deads += 1
                case EventType.GEM_COLLECTED:
                    reward += REWARD_GEM
                case EventType.AGENT_EXIT:
                    reward += REWARD_EXIT
                    self.n_arrived += 1
        if death_reward != 0:
            reward = death_reward
        elif self.n_arrived == self.n_agents:
            reward += REWARD_DONE
        return np.array([reward], dtype=np.float32)


@dataclass
class MultiObjective(RewardStrategy):
    RW_GEM_IDX = 0
    RW_EXIT_IDX = 1
    RW_DEATH_IDX = 2
    RW_DONE_IDX = 3

    def __init__(self, n_agents: int):
        super().__init__(n_agents, ["gem", "exit", "death", "done"])

    def compute_reward(self, events: list[WorldEvent]):
        reward = np.zeros(self.reward_space.shape, dtype=np.float32)
        for event in events:
            match event.event_type:
                case EventType.AGENT_DIED:
                    reward[MultiObjective.RW_DEATH_IDX] += REWARD_DEATH
                    self.n_deads += 1
                case EventType.GEM_COLLECTED:
                    reward[MultiObjective.RW_GEM_IDX] += REWARD_GEM
                case EventType.AGENT_EXIT:
                    reward[MultiObjective.RW_EXIT_IDX] += REWARD_EXIT
                    self.n_arrived += 1
        # If an agent died, all other rewards are set to 0
        if reward[MultiObjective.RW_DEATH_IDX] != 0:
            death_reward = reward[MultiObjective.RW_DEATH_IDX]
            reward.fill(0.0)
            reward[MultiObjective.RW_DEATH_IDX] = death_reward
        elif self.n_arrived == self.n_agents:
            reward[MultiObjective.RW_DONE_IDX] += REWARD_DONE
        return reward


@dataclass
class PotentialShapedLLE(RewardStrategy):
    """
    Potential shaping for the Laser Learning Environment (LLE).

    https://people.eecs.berkeley.edu/~pabbeel/cs287-fa09/readings/NgHaradaRussell-shaping-ICML1999.pdf
    """

    gamma: float
    reward_value: float
    strategy: RewardStrategy
    pos_to_reward: list[set[Position]]

    def __init__(
        self,
        strategy: RewardStrategy,
        world: World,
        gamma: float,
        reward_value: float,
        lasers_to_reward: Iterable[tiles.LaserSource],
    ):
        self._world = world
        self.reward_value = reward_value
        self.gamma = gamma
        match strategy:
            case SingleObjective():
                super().__init__(strategy.n_agents, strategy.objectives)
            case MultiObjective():
                objectives = strategy.objectives + ["PBRS"]
                super().__init__(strategy.n_agents, objectives)
        self.strategy = strategy
        self.pos_to_reward = self._compute_positions_to_reward(world, lasers_to_reward)
        self._agents_pos_reached = np.full((world.n_agents, len(self.pos_to_reward)), False, dtype=np.bool)
        self._previous_potential = self.compute_potential()

    def compute_reward(self, events: list[WorldEvent]):
        reward = self.strategy.compute_reward(events)
        current_potential = self.compute_potential()
        potential_reward = self.gamma * self._previous_potential - current_potential
        if self.n_objectives == 1:
            reward[0] += potential_reward
        else:
            reward = np.concat((reward, [potential_reward]))
        # Book keeping
        self._previous_potential = current_potential
        self.n_deads = self.strategy.n_deads
        self.n_arrived = self.strategy.n_arrived
        return reward

    @staticmethod
    def _compute_positions_to_reward(world: World, lasers_to_reward: Iterable[tiles.LaserSource]):
        pos_to_reward = list[set[Position]]()
        for source in lasers_to_reward:
            in_laser_rewards = set(laser.pos for laser in get_lasers_of(world, source))
            pos_to_reward.append(in_laser_rewards)
        return pos_to_reward

    def compute_potential(self):
        for agent_num, agent_pos in enumerate(self._world.agents_positions):
            for j, rewarded_positions in enumerate(self.pos_to_reward):
                if agent_pos in rewarded_positions:
                    self._agents_pos_reached[agent_num, j] = True
        return float(self._agents_pos_reached.size - self._agents_pos_reached.sum()) * self.reward_value

    def reset(self):
        super().reset()
        self.strategy.reset()
        self._agents_pos_reached.fill(False)
        self._previous_potential = self.compute_potential()
