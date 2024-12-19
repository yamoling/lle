from dataclasses import dataclass
from abc import abstractmethod, ABC

import numpy as np
import numpy.typing as npt
from marlenv import DiscreteSpace

from lle import WorldEvent, EventType

REWARD_GEM = 1.0
REWARD_EXIT = 1.0
REWARD_DONE = 1.0
REWARD_DEATH = -1.0


@dataclass
class RewardStrategy(ABC):
    objectives: list[str]
    n_agents: int

    def __init__(self, n_agents: int, objectives: list[str]):
        self.objectives = objectives
        self.reward_space = DiscreteSpace(len(objectives), objectives)
        self.n_agents = n_agents
        self.n_arrived = 0
        self.n_deads = 0

    @property
    def n_objectives(self) -> int:
        return self.reward_space.shape[0]

    def reset(self):
        self.n_arrived = 0
        self.n_deads = 0

    @abstractmethod
    def compute_reward(self, events: list[WorldEvent]) -> npt.NDArray[np.float32]:
        """Compute the reward for the given events."""


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
