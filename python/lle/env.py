from typing import Any, Literal
from typing_extensions import override
from dataclasses import dataclass
from serde import serde
import cv2
import numpy as np

from lle import World, Action, EventType, WorldState, REWARD_AGENT_DIED, REWARD_AGENT_EXIT, REWARD_END_GAME, REWARD_GEM_COLLECTED
from rlenv import RLEnv, DiscreteActionSpace, Observation, StepData
from .observations import ObservationType


@serde
@dataclass
class LLE(RLEnv[DiscreteActionSpace]):
    """Laser Learning Environment (LLE)"""

    obs_type: str

    def __init__(self, world: World, obs_type: ObservationType | str = ObservationType.STATE):
        self.world = world
        super().__init__(DiscreteActionSpace(world.n_agents, Action.N, [a.name for a in Action.ALL]))
        if isinstance(obs_type, str):
            obs_type = ObservationType.from_str(obs_type)
        self.obs_type = obs_type.name
        self.world_observer = obs_type.get_observation_generator(self.world)
        self._state_observer = ObservationType.FLATTENED.get_observation_generator(self.world)
        self._obs_type = obs_type

        self.done = False
        self.n_arrived = 0

    @property
    def width(self) -> int:
        return self.world.width

    @property
    def height(self) -> int:
        return self.world.height

    @property
    def state_shape(self):
        return self.get_state().shape

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
        assert not self.done, "Can not play when the game is done !"
        events = self.world.step([Action(a) for a in actions])
        reward = 0.0
        death_reward = 0.0
        for event in events:
            match event.event_type:
                case EventType.AGENT_DIED:
                    death_reward += REWARD_AGENT_DIED
                    self.done = True
                case EventType.GEM_COLLECTED:
                    reward += REWARD_GEM_COLLECTED
                case EventType.AGENT_EXIT:
                    reward += REWARD_AGENT_EXIT
                    self.n_arrived += 1
        if death_reward != 0:
            reward = death_reward
        elif self.n_arrived == self.n_agents:
            reward += REWARD_END_GAME
            self.done = True

        obs_data = self.world_observer.observe()
        obs = Observation(obs_data, self.available_actions(), self.get_state())
        info = {"gems_collected": self.world.gems_collected, "exit_rate": self.n_arrived / self.n_agents}
        return StepData(obs, reward, self.done, False, info)

    @override
    def reset(self):
        self.world.reset()
        self.done = False
        self.n_arrived = 0
        obs = self.world_observer.observe()
        return Observation(obs, self.available_actions(), self.get_state())

    @override
    def get_state(self) -> np.ndarray[np.float32, Any]:
        state = self.world.get_state()
        gems_collected = np.array(state.gems_collected, dtype=np.float32)
        agents_positions = np.array(state.agents_positions, dtype=np.float32).reshape(-1)
        return np.concatenate([gems_collected, agents_positions])

    @override
    def render(self, mode: Literal["human", "rgb_array"] = "human"):
        image = self.world.get_image()
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
        env = LLE(World.level(level), obs_type)
        env.name = f"{env.name}-lvl{level}"
        return env

    def seed(self, _seed_value: int):
        # There is nothing random in the world to seed.
        return

    def set_state(self, state: WorldState):
        self.world.set_state(state)
        agents = self.world.agents
        self.done = any(agent.is_dead for agent in agents) or all(agent.has_arrived for agent in agents)
        self.n_arrived = sum(agent.has_arrived for agent in agents)
