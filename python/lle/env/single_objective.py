from dataclasses import dataclass

import numpy as np
import numpy.typing as npt
from marlenv import DiscreteActionSpace, MARLEnv

from lle import EventType, WorldEvent, WorldState

from .core import REWARD_DEATH, REWARD_DONE, REWARD_EXIT, REWARD_GEM, Core


@dataclass
class SOLLE(MARLEnv[DiscreteActionSpace, npt.NDArray[np.float32], npt.NDArray[np.float32], float]):
    """
    Single Objective Laser Learning Environment (SOLLE)
    """

    name: str
    width: int
    height: int

    def __init__(self, core: Core):
        self.world = core.world
        self.core = core
        super().__init__(action_space=core.action_space, observation_shape=core.observation_shape, state_shape=core.state_shape)
        self.name = SOLLE.__name__
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
        reward = 0.0
        death_reward = 0.0
        for event in events:
            match event.event_type:
                case EventType.AGENT_DIED:
                    reward += REWARD_DEATH
                case EventType.GEM_COLLECTED:
                    reward += REWARD_GEM
                case EventType.AGENT_EXIT:
                    reward += REWARD_EXIT
        if death_reward != 0:
            return death_reward
        if self.core.n_arrived == self.n_agents:
            reward += REWARD_DONE
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
