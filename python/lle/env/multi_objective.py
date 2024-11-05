from dataclasses import dataclass

import numpy as np
import numpy.typing as npt
from marlenv import MARLEnv, DiscreteActionSpace, DiscreteSpace

from lle import EventType, WorldEvent, WorldState

from .core import REWARD_DEATH, REWARD_DONE, REWARD_EXIT, REWARD_GEM, Core

# Reward indices for multi-objective LLE
RW_DEATH_IDX = 0
RW_GEM_IDX = 1
RW_EXIT_IDX = 2
RW_DONE_IDX = 3


@dataclass
class MOLLE(MARLEnv[DiscreteActionSpace, npt.NDArray[np.float32], npt.NDArray[np.float32]]):
    """
    Multi-Objective Laser Learning Environment (MO LLE)
    """

    name: str
    width: int
    height: int

    def __init__(self, core: Core):
        self.world = core.world
        self.core = core
        super().__init__(
            action_space=core.action_space,
            observation_shape=core.observation_shape,
            state_shape=core.state_shape,
            reward_space=DiscreteSpace(4, ["death", "gem", "exit", "done"]),
        )
        self.name = MOLLE.__name__
        self.width = self.world.width
        self.height = self.world.height

    @property
    def done(self):
        return self.core.done

    @property
    def agent_state_size(self) -> int:
        return self.core.agent_state_size

    def reset(self):
        return self.core.reset()

    def compute_reward(self, events: list[WorldEvent]):
        reward = np.zeros((4,), dtype=np.float32)
        for event in events:
            match event.event_type:
                case EventType.AGENT_DIED:
                    reward[RW_DEATH_IDX] += REWARD_DEATH
                case EventType.GEM_COLLECTED:
                    reward[RW_GEM_IDX] += REWARD_GEM
                case EventType.AGENT_EXIT:
                    reward[RW_EXIT_IDX] += REWARD_EXIT
        if self.core.n_arrived == self.n_agents:
            reward[RW_DONE_IDX] += REWARD_DONE
        # If an agent died, all other rewards are set to 0
        if reward[RW_DEATH_IDX] != 0:
            death_reward = reward[RW_DEATH_IDX]
            reward = np.zeros((4,), dtype=np.float32)
            reward[RW_DEATH_IDX] = death_reward
        return reward

    def available_actions(self):
        return self.core.available_actions()

    def step(self, actions):
        obs, done, info, events = self.core.step(actions)
        reward = self.compute_reward(events)
        return obs, reward, done, False, info

    def get_state(self):
        return self.core.get_state()

    def render(self, mode):
        return self.core.render(mode)

    def set_state(self, state: WorldState):
        self.core.set_state(state)

    def seed(self, seed_value: int):
        return self.core.seed(seed_value)
