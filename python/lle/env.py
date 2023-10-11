from typing import Any, Literal
from typing_extensions import override
from lle import World, Action
import cv2
import numpy as np

from rlenv import RLEnv, DiscreteActionSpace, Observation
from .observations import ObservationType


class LLE(RLEnv[DiscreteActionSpace]):
    """Laser Learning Environment (LLE)"""

    def __init__(self, world: World, obs_type: ObservationType | str = ObservationType.STATE):
        self.world = world
        super().__init__(DiscreteActionSpace(self.world.n_agents, Action.N, [a.name for a in Action.ALL]))
        if isinstance(obs_type, str):
            obs_type = ObservationType.from_str(obs_type)
        self.world_observer = obs_type.get_observation_generator(self.world)
        self._state_observer = ObservationType.FLATTENED.get_observation_generator(self.world)
        self._obs_type = obs_type

    @property
    def width(self) -> int:
        return self.world.width

    @property
    def height(self) -> int:
        return self.world.height

    @property
    def state_shape(self):
        return self._state_observer.shape

    @property
    def observation_shape(self):
        return self.world_observer.shape

    @override
    def available_actions(self) -> np.ndarray[np.int32, Any]:
        available_actions = np.zeros((self.n_agents, self.n_actions), dtype=np.int64)
        for agent, actions in enumerate(self.world.available_actions()):
            for action in actions:
                available_actions[agent, action.value] = 1
        return available_actions

    @override
    def step(self, actions: np.ndarray[np.int32, Any]):
        reward = self.world.step([Action(a) for a in actions])
        obs_data = self.world_observer.observe()
        obs = Observation(obs_data, self.available_actions(), self.get_state())
        info = {"gems_collected": self.world.gems_collected, "exit_rate": self.world.exit_rate}
        return obs, reward, self.world.done, False, info

    @override
    def reset(self):
        self.world.reset()
        obs = self.world_observer.observe()
        return Observation(obs, self.available_actions(), self.get_state())

    @override
    def get_state(self) -> np.ndarray[np.float32, Any]:
        return self._state_observer.observe()[0]

    @override
    def render(self, mode: Literal["human", "rgb_array"] = "human"):
        image = self.world.get_image()
        # image = cv2.cvtColor(image, cv2.COLOR_BGR2RGB)
        match mode:
            case "human":
                cv2.imshow("LLE", image)
                cv2.waitKey(1)
            case "rgb_array":
                return image
            case other:
                raise NotImplementedError(f"Rendering mode not implemented: {other}")

    @staticmethod
    def from_str(world_string: str, obs_type: ObservationType = ObservationType.FLATTENED) -> "LLE":
        return LLE(World(world_string), obs_type)

    @staticmethod
    def from_file(path: str, obs_type: ObservationType = ObservationType.FLATTENED) -> "LLE":
        return LLE(World.from_file(path), obs_type)

    @staticmethod
    def level(level: int, obs_type: ObservationType = ObservationType.FLATTENED) -> "LLE":
        """Load a level from the levels folder"""
        if level <= 0 or level > 6:
            raise NotImplementedError("Only levels 1-6 are implemented")
        return LLE(World.from_file(f"level{level}"), obs_type)

    def seed(self, _seed_value: int):
        return
