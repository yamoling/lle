from enum import IntEnum
from typing import Literal, Sequence, TypeVar, Optional
from abc import abstractmethod

import numpy as np
import numpy.typing as npt
from marlenv import Observation, DiscreteActionSpace, State, MARLEnv, Step, DiscreteSpace

from lle import Action, EventType, World, WorldEvent, WorldState
from lle.observations import ObservationType, StateGenerator

from .builder import Builder

REWARD_DEATH = -1.0
REWARD_GEM = 1.0
REWARD_EXIT = 1.0
REWARD_DONE = 1.0

R = TypeVar("R", bound=float | npt.NDArray[np.float32])


class DeathStrategy(IntEnum):
    END = 0
    """End the game when an agent dies"""
    RESPAWN = 1
    """The agent respawns on its start position when it dies"""


class LLE(MARLEnv[DiscreteActionSpace, npt.NDArray[np.float32], npt.NDArray[np.float32], R]):
    """Laser Learning Environment (LLE)"""

    name: str
    obs_type: str
    state_type: str
    death_strategy: DeathStrategy
    n_agents: int
    n_actions: int
    action_space: DiscreteActionSpace

    def __init__(
        self,
        world: World,
        reward_space: Optional[DiscreteSpace],
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
        super().__init__(
            DiscreteActionSpace(self.world.n_agents, Action.N, [a.name for a in Action.ALL]),
            self.observation_generator.shape,
            self.get_state().shape,
            reward_space=reward_space,
        )

        match death_strategy:
            case "end":
                self.death_strategy = DeathStrategy.END
            case "respawn":
                self.death_strategy = DeathStrategy.RESPAWN
            case other:
                raise ValueError(f"Unknown death strategy: {other}")
        self.walkable_lasers = walkable_lasers
        self.n_agents = world.n_agents
        self.n_arrived = 0
        self.done = False

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

    def step(self, actions: np.ndarray | Sequence[int]):
        if self.done:
            raise ValueError("Cannot step in a done environment")
        agents_actions = [Action(a) for a in actions]
        events = self.world.step(agents_actions)
        self._update(events)
        return Step(
            self.get_observation(),
            self.get_state(),
            reward=self.compute_reward(events),
            done=self.done,
            info={"gems_collected": self.world.gems_collected, "exit_rate": self.n_arrived / self.n_agents},
        )

    def _update(self, events: list[WorldEvent]):
        for event in events:
            match event.event_type:
                case EventType.AGENT_DIED:
                    if self.death_strategy == DeathStrategy.END:
                        self.done = True
                    else:
                        raise NotImplementedError("Respawn strategy is not implemented yet")
                case EventType.AGENT_EXIT:
                    self.n_arrived += 1
                    if self.n_arrived == self.n_agents:
                        self.done = True

    @abstractmethod
    def compute_reward(self, events: list[WorldEvent]) -> R:
        """Compute the reward according to the events that occurred"""

    def reset(self):
        self.world.reset()
        self.n_arrived = 0
        self.done = False
        return self.get_observation(), self.get_state()

    def get_state(self):
        # We assume that the state is the same as the observation of the first agent
        # of the observation generator.
        return State(self.state_generator.observe()[0])

    def set_state(self, state: State[npt.NDArray[np.float32]] | WorldState):
        if isinstance(state, WorldState):
            world_state = state
        else:
            world_state = self.state_generator.to_world_state(state.data)
        self.n_arrived = 0
        self.done = False
        events = self.world.set_state(world_state)
        self._update(events)

    def get_observation(self):
        return Observation(self.observation_generator.observe(), self.available_actions())

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

    def seed(self, seed_value: int):
        return self.world.seed(seed_value)

    def get_image(self):
        return self.world.get_image()
