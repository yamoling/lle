from typing import Any, Literal, ClassVar, Sequence
from typing_extensions import override
from dataclasses import dataclass
from serde import serde
import cv2
import numpy as np

from lle import World, Action, EventType, WorldState, WorldEvent
from rlenv import RLEnv, DiscreteActionSpace, Observation, DiscreteSpace
from .observations import ObservationType, StateGenerator


@serde
@dataclass
class LLE(RLEnv[DiscreteActionSpace]):
    """Laser Learning Environment (LLE)"""

    obs_type: str
    state_type: str
    multi_objective: bool

    REWARD_DEATH: ClassVar[float] = -1.0
    REWARD_GEM: ClassVar[float] = 1.0
    REWARD_EXIT: ClassVar[float] = 1.0
    REWARD_DONE: ClassVar[float] = 1.0

    RW_DEATH_IDX = 0
    RW_GEM_IDX = 1
    RW_EXIT_IDX = 2
    RW_DONE_IDX = 3

    def __init__(
        self,
        world: World,
        obs_type: ObservationType = ObservationType.STATE,
        state_type: ObservationType = ObservationType.STATE,
        multi_objective: bool = False,
    ):
        self.multi_objective = multi_objective
        self.world = world
        self.obs_type = obs_type.name
        self.state_type = state_type.name
        self.observation_generator = obs_type.get_observation_generator(world)
        self.state_generator = state_type.get_observation_generator(world)
        if multi_objective:
            reward_space = DiscreteSpace(4, ["death", "gem", "exit", "done"])
        else:
            reward_space = None
        super().__init__(
            DiscreteActionSpace(world.n_agents, Action.N, [a.name for a in Action.ALL]),
            observation_shape=self.observation_generator.shape,
            state_shape=self.get_state().shape,
            reward_space=reward_space,
        )
        self.done = False
        self.n_arrived = 0

    @property
    def width(self) -> int:
        return self.world.width

    @property
    def height(self) -> int:
        return self.world.height

    @property
    def agent_state_size(self):
        match self.state_generator:
            case StateGenerator():
                return self.state_generator.unit_size
            case other:
                raise ValueError(f"State type {other} does not support `unit_state_size`.")

    def get_observation(self):
        return Observation(self.observation_generator.observe(), self.available_actions(), self.get_state())

    @override
    def available_actions(self) -> np.ndarray[np.int32, Any]:
        available_actions = np.zeros((self.n_agents, self.n_actions), dtype=np.int64)
        for agent, actions in enumerate(self.world.available_actions()):
            for action in actions:
                available_actions[agent, action.value] = 1
        return available_actions

    @override
    def step(self, actions: Sequence[int] | np.ndarray[np.int32, Any]):
        assert not self.done, "Can not play when the game is done !"
        events = self.world.step([Action(a) for a in actions])

        for event in events:
            match event.event_type:
                case EventType.AGENT_DIED:
                    self.done = True
                case EventType.AGENT_EXIT:
                    self.n_arrived += 1
        if self.n_arrived == self.n_agents:
            self.done = True
        if self.multi_objective:
            reward = self._reward_multi_objective(events)
        else:
            reward = self._reward_classic(events)

        info = {"gems_collected": self.world.gems_collected, "exit_rate": self.n_arrived / self.n_agents}
        return self.get_observation(), reward, self.done, False, info

    def _reward_classic(self, events: list[WorldEvent]):
        reward = 0.0
        death_reward = 0.0
        for event in events:
            match event.event_type:
                case EventType.AGENT_DIED:
                    reward += LLE.REWARD_DEATH
                case EventType.GEM_COLLECTED:
                    reward += LLE.REWARD_GEM
                case EventType.AGENT_EXIT:
                    reward += LLE.REWARD_EXIT
        if death_reward != 0:
            reward = death_reward
        elif self.n_arrived == self.n_agents:
            reward += LLE.REWARD_DONE
        return np.array([reward], dtype=np.float32)

    def _reward_multi_objective(self, events: list[WorldEvent]):
        reward = np.zeros((self.reward_size), dtype=np.float32)
        for event in events:
            match event.event_type:
                case EventType.AGENT_DIED:
                    reward[LLE.RW_DEATH_IDX] += LLE.REWARD_DEATH
                case EventType.GEM_COLLECTED:
                    reward[LLE.RW_GEM_IDX] += LLE.REWARD_GEM
                case EventType.AGENT_EXIT:
                    reward[LLE.RW_EXIT_IDX] += LLE.REWARD_EXIT
        if self.n_arrived == self.n_agents:
            reward[LLE.RW_DONE_IDX] += LLE.REWARD_DONE
        # If an agent died, all other rewards are set to 0
        if reward[LLE.RW_DEATH_IDX] != 0:
            death_reward = reward[LLE.RW_DEATH_IDX]
            reward = np.zeros((self.reward_size), dtype=np.float32)
            reward[LLE.RW_DEATH_IDX] = death_reward
        return reward

    @override
    def reset(self):
        self.world.reset()
        self.done = False
        self.n_arrived = 0
        return self.get_observation()

    @override
    def get_state(self) -> np.ndarray[np.float32, Any]:
        # We assume that the state is the same as the observation of the first agent
        # of the observation generator.
        return self.state_generator.observe()[0]

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
    def from_str(
        world_string: str,
        obs_type: ObservationType = ObservationType.FLATTENED,
        state_type: ObservationType = ObservationType.STATE,
        multi_objective: bool = False,
    ):
        return LLE(World(world_string), obs_type, state_type, multi_objective)

    @staticmethod
    def from_file(
        path: str,
        obs_type: ObservationType = ObservationType.FLATTENED,
        state_type: ObservationType = ObservationType.STATE,
        multi_objective: bool = False,
    ):
        import os

        env = LLE(World.from_file(path), obs_type, state_type, multi_objective)
        filename = os.path.basename(path)
        env.name = f"{env.name}-{filename}"
        return env

    @staticmethod
    def level(
        level: int,
        obs_type: ObservationType = ObservationType.FLATTENED,
        state_type: ObservationType = ObservationType.STATE,
        multi_objective: bool = False,
    ):
        """Load a level from the levels folder"""
        env = LLE(World.level(level), obs_type, state_type, multi_objective)
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

    @property
    def max_score(self):
        return self.n_agents * LLE.REWARD_EXIT + LLE.REWARD_DONE + LLE.REWARD_GEM * self.world.n_gems
