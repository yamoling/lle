from typing import Literal, Optional
from typing_extensions import override
from dataclasses import dataclass
import cv2
from enum import IntEnum
import numpy as np
import numpy.typing as npt

from lle import World, Action, EventType, WorldState
from rlenv import RLEnv, DiscreteActionSpace, Observation
from lle.observations import ObservationType, StateGenerator
from .reward_strategy import RewardStrategy, MultiObjective, SingleObjective


class DeathStrategy(IntEnum):
    END = 0
    STAY = 1
    RESPAWN = 2


@dataclass
class Builder:
    _env_name: str
    _world: World
    _obs_type: ObservationType
    _state_type: ObservationType
    _multi_objective: bool
    _death_strategy: DeathStrategy
    _reward_strategy: RewardStrategy
    _walkable_lasers: bool

    def __init__(self, world: World):
        self._world = world
        self._obs_type = ObservationType.LAYERED
        self._state_type = ObservationType.STATE
        self._death_strategy = DeathStrategy.END
        self._reward_strategy = SingleObjective(world.n_agents)
        self._multi_objective = False
        self._env_name = LLE.__name__
        self._walkable_lasers = True

    def obs_type(self, obs_type: ObservationType):
        """
        Set the observation type of the environment (set to ObservationType.LAYERED by default).
        """
        self._obs_type = obs_type
        return self

    def state_type(self, state_type: ObservationType):
        """
        Set the state type of the environment (set to ObservationType.STATE by default).
        """
        self._state_type = state_type
        return self

    def walkable_lasers(self, walkable_lasers: bool):
        """
        Set wheter the agents can walk on active laser of different color
        Agent can still die if standing on a disabled laser tile that is reactivated
        """
        self._walkable_lasers = walkable_lasers
        return self

    def multi_objective(self):
        """Transform the reward strategy to a multi-objective one (single objective by default)."""
        self._reward_strategy = MultiObjective(self._world.n_agents)
        self._multi_objective = True
        return self

    def death_strategy(self, death_strategy: Literal["respawn", "end", "stay"]):
        """Set the behaviour of the agents when they die (end the episode by default)."""
        match death_strategy:
            case "respawn":
                # TODO: agents should not be able to go on an other agent's respawn position
                raise NotImplementedError("Respawn strategy is not implemented yet.")
            case "end":
                self._death_strategy = DeathStrategy.END
            case "stay":
                self._death_strategy = DeathStrategy.STAY
            case other:
                raise ValueError(f"Invalid death strategy: {other}")
        return self

    def name(self, name: str):
        """Set the name of the environment."""
        self._env_name = name
        return self

    def build(self):
        return LLE(
            name=self._env_name,
            world=self._world,
            obs_type=self._obs_type,
            state_type=self._state_type,
            reward_strategy=self._reward_strategy,
            death_strategy=self._death_strategy,
            walkable_lasers=self._walkable_lasers,
        )


@dataclass
class LLE(RLEnv[DiscreteActionSpace]):
    """Laser Learning Environment (LLE)"""

    obs_type: str
    state_type: str
    death_strategy: DeathStrategy
    reward_strategy: RewardStrategy

    def __init__(
        self,
        world: World,
        name: str = "LLE",
        obs_type: ObservationType = ObservationType.STATE,
        state_type: ObservationType = ObservationType.STATE,
        reward_strategy: Optional[RewardStrategy] = None,
        death_strategy: DeathStrategy = DeathStrategy.END,
        walkable_lasers: bool = True,
    ):
        self.world = world
        self.obs_type = obs_type.name
        self.state_type = state_type.name
        self.observation_generator = obs_type.get_observation_generator(world)
        self.state_generator = state_type.get_observation_generator(world)
        self.death_strategy = death_strategy
        self.reward_strategy = reward_strategy or SingleObjective(world.n_agents)
        self.walkable_lasers = walkable_lasers

        super().__init__(
            DiscreteActionSpace(world.n_agents, Action.N, [a.name for a in Action.ALL]),
            observation_shape=self.observation_generator.shape,
            state_shape=self.get_state().shape,
            reward_space=self.reward_strategy.reward_space,
        )
        self.name = name
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
                raise NotImplementedError(f"State type {other} does not support `unit_state_size`.")

    def get_observation(self):
        return Observation(self.observation_generator.observe(), self.available_actions(), self.get_state())

    @override
    def available_actions(self):
        available_actions = np.full((self.n_agents, self.n_actions), False, dtype=bool)
        lasers = self.world.lasers
        agents_pos = self.world.agents_positions
        for agent, actions in enumerate(self.world.available_actions()):
            for action in actions:
                if not self.walkable_lasers:
                    agent_pos = agents_pos[agent]
                    new_pos = (agent_pos[0] + action.delta[0], agent_pos[1] + action.delta[1])
                    # ignore action if new position is an active laser of another color
                    if any(laser_pos == new_pos and laser.agent_id != agent and laser.is_on for laser_pos, laser in lasers):
                        continue
                available_actions[agent, action.value] = True
        return available_actions

    @override
    def step(self, actions: np.ndarray):
        assert not self.done, "Can not play when the game is done !"
        agents_actions = [Action(a) for a in actions]
        prev_positions = self.world.agents_positions
        events = self.world.step(agents_actions)
        set_state = None
        reset = False

        for event in events:
            match event.event_type:
                case EventType.AGENT_DIED:
                    match self.death_strategy:
                        case DeathStrategy.END:
                            self.done = True
                        case DeathStrategy.STAY:
                            if set_state is None:
                                set_state = self.world.get_state()
                            # Changing the correct index in-place does not work
                            new_positions = set_state.agents_positions
                            new_positions[event.agent_id] = prev_positions[event.agent_id]
                            set_state.agents_positions = new_positions
                case EventType.AGENT_EXIT:
                    self.n_arrived += 1
        if set_state is not None:
            self.world.set_state(set_state)
        if reset:
            self.reset()
        if self.n_arrived == self.n_agents:
            self.done = True
        reward = self.reward_strategy.compute_reward(events)

        info = {"gems_collected": self.world.gems_collected, "exit_rate": self.n_arrived / self.n_agents}
        return self.get_observation(), reward, self.done, False, info

    @override
    def reset(self):
        self.world.reset()
        self.reward_strategy.reset()
        self.done = False
        self.n_arrived = 0
        return self.get_observation()

    @override
    def get_state(self) -> npt.NDArray[np.float32]:
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
    def from_str(world_string: str):
        return Builder(World(world_string))

    @staticmethod
    def from_file(path: str):
        import os

        return Builder(World.from_file(path)).name(f"LLE-{os.path.basename(path)}")

    @staticmethod
    def level(level: int):
        """Load a level from the levels folder"""
        return Builder(World.level(level)).name(f"LLE-lvl{level}")

    def seed(self, _seed_value: int):
        # There is nothing random in the world to seed.
        return

    def set_state(self, state: WorldState):
        self.reward_strategy.reset()
        events = self.world.set_state(state)
        # Don't do anything with the events, just update the reward strategy
        self.reward_strategy.compute_reward(events)
        agents = self.world.agents
        self.done = any(agent.is_dead for agent in agents) or all(agent.has_arrived for agent in agents)
        self.n_arrived = sum(agent.has_arrived for agent in agents)
