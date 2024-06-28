from abc import ABC, abstractmethod
import numpy as np
import numpy.typing as npt
from lle import WorldEvent, EventType
from dataclasses import dataclass
from rlenv import DiscreteSpace


@dataclass
class RewardStrategy(ABC):
    name: str
    reward_space: DiscreteSpace

    def __init__(self, space: DiscreteSpace):
        self.name = self.__class__.__name__
        self.reward_space = space

    @abstractmethod
    def reset(self): ...

    @abstractmethod
    def compute_reward(self, events: list[WorldEvent]) -> npt.NDArray[np.float32]: ...


REWARD_DEATH = -1.0
REWARD_GEM = 1.0
REWARD_EXIT = 1.0
REWARD_DONE = 1.0

# Reward indices for multi-objective LLE
RW_DEATH_IDX = 0
RW_GEM_IDX = 1
RW_EXIT_IDX = 2
RW_DONE_IDX = 3


class SingleObjective(RewardStrategy):
    def __init__(self, n_agents: int):
        super().__init__(DiscreteSpace(1, ["Default"]))
        self.n_agents = n_agents
        self.n_arrived = 0

    def compute_reward(self, events: list[WorldEvent]):
        reward = 0.0
        death_reward = 0.0
        for event in events:
            match event.event_type:
                case EventType.AGENT_DIED:
                    reward += REWARD_DEATH
                case EventType.GEM_COLLECTED:
                    reward += REWARD_GEM
                case EventType.AGENT_EXIT:
                    self.n_arrived += 1
                    reward += REWARD_EXIT
        if death_reward != 0:
            reward = death_reward
        elif self.n_arrived == self.n_agents:
            reward += REWARD_DONE
        return np.array([reward], dtype=np.float32)

    def reset(self):
        self.n_arrived = 0


class MultiObjective(RewardStrategy):
    def __init__(self, n_agents: int):
        super().__init__(DiscreteSpace(4, ["death", "gem", "exit", "done"]))
        self.n_agents = n_agents
        self.n_arrived = 0

    def reset(self):
        self.n_arrived = 0

    def compute_reward(self, events: list[WorldEvent]):
        reward = np.zeros((4,), dtype=np.float32)
        for event in events:
            match event.event_type:
                case EventType.AGENT_DIED:
                    reward[RW_DEATH_IDX] += REWARD_DEATH
                case EventType.GEM_COLLECTED:
                    reward[RW_GEM_IDX] += REWARD_GEM
                case EventType.AGENT_EXIT:
                    self.n_arrived += 1
                    reward[RW_EXIT_IDX] += REWARD_EXIT
        if self.n_arrived == self.n_agents:
            reward[RW_DONE_IDX] += REWARD_DONE
        # If an agent died, all other rewards are set to 0
        if reward[RW_DEATH_IDX] != 0:
            death_reward = reward[RW_DEATH_IDX]
            reward = np.zeros((4,), dtype=np.float32)
            reward[RW_DEATH_IDX] = death_reward
        return reward
