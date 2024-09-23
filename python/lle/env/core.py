from dataclasses import dataclass
from enum import IntEnum
from typing import Literal, TypeVar

import cv2
import numpy as np
import numpy.typing as npt
from marlenv import Observation, DiscreteActionSpace

from lle import Action, EventType, World, WorldState
from lle.observations import ObservationType, StateGenerator

from .builder import Builder

REWARD_DEATH = -1.0
REWARD_GEM = 1.0
REWARD_EXIT = 1.0
REWARD_DONE = 1.0

R = TypeVar("R", bound=float | npt.NDArray[np.float32])


class DeathStrategy(IntEnum):
    END = 0
    STAY = 1
    RESPAWN = 2


@dataclass
class Core:
    """Laser Learning Environment (LLE)"""

    obs_type: str
    state_type: str
    death_strategy: DeathStrategy
    n_agents: int
    n_actions: int
    action_space: DiscreteActionSpace

    def __init__(
        self,
        world: World,
        obs_type: ObservationType = ObservationType.STATE,
        state_type: ObservationType = ObservationType.STATE,
        death_strategy: Literal["respawn", "end", "stay"] = "end",
        walkable_lasers: bool = True,
    ):
        self.world = world
        self.obs_type = obs_type.name
        self.state_type = state_type.name
        self.observation_generator = obs_type.get_observation_generator(world)
        self.state_generator = state_type.get_observation_generator(world)
        match death_strategy:
            case "end":
                self.death_strategy = DeathStrategy.END
            case "respawn":
                self.death_strategy = DeathStrategy.RESPAWN
            case "stay":
                self.death_strategy = DeathStrategy.STAY
            case other:
                raise ValueError(f"Unknown death strategy: {other}")
        self.walkable_lasers = walkable_lasers
        self.n_actions = Action.N
        self.n_agents = world.n_agents
        self.action_space = DiscreteActionSpace(self.world.n_agents, Action.N, [a.name for a in Action.ALL])
        self.state_shape = self.get_state().shape
        self.observation_shape = self.observation_generator.shape
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

    def available_actions(self):
        available_actions = np.full((self.world.n_agents, self.n_actions), False, dtype=bool)
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

    def step(self, actions: np.ndarray | list[int]):
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
        if self.n_arrived == self.world.n_agents:
            self.done = True
        info = {"gems_collected": self.world.gems_collected, "exit_rate": self.n_arrived / self.n_agents}
        return self.get_observation(), self.done, info, events

    def reset(self):
        self.world.reset()
        self.done = False
        self.n_arrived = 0
        return self.get_observation()

    def get_state(self) -> npt.NDArray[np.float32]:
        # We assume that the state is the same as the observation of the first agent
        # of the observation generator.
        return self.state_generator.observe()[0]

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
        # self.reward_strategy.reset()
        _events = self.world.set_state(state)
        # Don't do anything with the events, just update the reward strategy
        # self.reward_strategy.compute_reward(events)
        agents = self.world.agents
        self.done = any(agent.is_dead for agent in agents) or all(agent.has_arrived for agent in agents)
        self.n_arrived = sum(agent.has_arrived for agent in agents)
