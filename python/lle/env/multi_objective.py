from typing import Literal
import numpy as np
import numpy.typing as npt
from marlenv import DiscreteSpace, DiscreteMARLEnv

from lle import EventType, WorldEvent, WorldState, World
from lle.observations import ObservationType

from .env import REWARD_DEATH, REWARD_DONE, REWARD_EXIT, REWARD_GEM, LLE

# Reward indices for multi-objective LLE
RW_DEATH_IDX = 0
RW_GEM_IDX = 1
RW_EXIT_IDX = 2
RW_DONE_IDX = 3


class MOLLE(LLE[npt.NDArray[np.float32]]):
    """
    Multi-Objective Laser Learning Environment (MO LLE)
    """

    def __init__(
        self,
        world: World,
        obs_type: ObservationType = ObservationType.STATE,
        state_type: ObservationType = ObservationType.STATE,
        death_strategy: Literal["respawn", "end", "stay"] = "end",
        walkable_lasers: bool = True,
    ):
        super().__init__(world, DiscreteSpace(4, ["death", "gem", "exit", "done"]), obs_type, state_type, death_strategy, walkable_lasers)

    def compute_reward(self, events: list[WorldEvent]):
        reward = np.zeros(self.reward_space.shape, dtype=np.float32)
        for event in events:
            match event.event_type:
                case EventType.AGENT_DIED:
                    reward[RW_DEATH_IDX] += REWARD_DEATH
                case EventType.GEM_COLLECTED:
                    reward[RW_GEM_IDX] += REWARD_GEM
                case EventType.AGENT_EXIT:
                    reward[RW_EXIT_IDX] += REWARD_EXIT
        if self.n_arrived == self.n_agents:
            reward[RW_DONE_IDX] += REWARD_DONE
        # If an agent died, all other rewards are set to 0
        if reward[RW_DEATH_IDX] != 0:
            death_reward = reward[RW_DEATH_IDX]
            reward = np.zeros((4,), dtype=np.float32)
            reward[RW_DEATH_IDX] = death_reward
        return reward
