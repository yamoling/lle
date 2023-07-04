from typing import Literal
from lle.lle import World
import cv2
import numpy as np

from rlenv import RLEnv, DiscreteActionSpace, Observation
from .lle import World, Action
from .observations import ObservationType


class LLE(RLEnv[DiscreteActionSpace]):
    def __init__(self, env_file: str, obs_type: ObservationType | str = ObservationType.RELATIVE_POSITIONS):
        self.world = World(env_file)
        super().__init__(DiscreteActionSpace(self.world.n_agents, Action.N, [a.name for a in Action.ALL]))
        if isinstance(obs_type, str):
            obs_type = ObservationType.from_str(obs_type)
        self.world_observer = obs_type.get_observation_generator(self.world)
        self._state_observer = ObservationType.FLATTENED.get_observation_generator(self.world)
        self._obs_type = obs_type
        dims = self.world.image_dimensions
        self._image_shape = (dims[1], dims[0], 3)
        self._image_byte_count = np.prod(self._image_shape)

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

    def get_avail_actions(self) -> np.ndarray[np.int64]:
        available_actions = np.zeros((self.n_agents, self.n_actions), dtype=np.int64)
        for agent, actions in enumerate(self.world.available_actions()):
            for action in actions:
                available_actions[agent, action.value] = 1
        return available_actions

    def step(self, actions: np.ndarray[np.int32]):
        actions = [Action(a) for a in actions]
        reward = self.world.step(actions)
        obs_data = self.world_observer.observe()
        obs = Observation(obs_data, self.get_avail_actions(), self.get_state())
        info = {"gems_collected": self.world.gems_collected, "exit_rate": self.world.exit_rate()}
        return obs, reward, self.world.done, False, info

    def reset(self):
        self.world.reset()
        obs = self.world_observer.observe()
        return Observation(obs, self.get_avail_actions(), self.get_state())

    def get_state(self) -> np.ndarray[np.float32]:
        return self._state_observer.observe()[0]

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

    def summary(self, static=False) -> dict[str,]:
        return {**super().summary(), "map_file_content": self.world.to_world_string(static)}
