from dataclasses import dataclass
from enum import IntEnum
from typing import Literal, Optional, Sequence

import numpy as np
import numpy.typing as npt
from marlenv import DiscreteActionSpace, MARLEnv, Observation, State, Step

from lle import Action, World, WorldState
from lle.observations import ObservationType, StateGenerator

from .builder import Builder
from .reward_strategy import RewardStrategy, SingleObjective


class DeathStrategy(IntEnum):
    END = 0
    """End the game when an agent dies"""
    RESPAWN = 1
    """The agent respawns on its start position when it dies"""

    @staticmethod
    def from_str(value: Literal["end", "respawn"]):
        match value:
            case "end":
                return DeathStrategy.END
            case "respawn":
                return DeathStrategy.RESPAWN
        raise ValueError(f"Unknown death strategy: {value}")


@dataclass
class LLE(MARLEnv[Sequence[int] | npt.NDArray, DiscreteActionSpace]):
    """Laser Learning Environment (LLE)"""

    obs_type: str
    state_type: str
    death_strategy: DeathStrategy
    walkable_lasers: bool
    reward_strategy: RewardStrategy

    def __init__(
        self,
        world: World,
        reward_strategy: Optional[RewardStrategy] = None,
        obs_type: ObservationType = ObservationType.STATE,
        state_type: ObservationType = ObservationType.STATE,
        name: Optional[str] = None,
        death_strategy: Literal["respawn", "end"] = "end",
        walkable_lasers: bool = True,
    ):
        self.world = world
        self.obs_type = obs_type.name
        self.state_type = state_type.name
        self.observation_generator = obs_type.get_observation_generator(world)
        self.state_generator = state_type.get_observation_generator(world)
        if reward_strategy is None:
            reward_strategy = SingleObjective(world.n_agents)
        self.reward_strategy = reward_strategy
        super().__init__(
            DiscreteActionSpace(self.world.n_agents, Action.N, [a.name for a in Action.ALL]),
            self.observation_generator.shape,
            self.get_state().shape,
            reward_space=self.reward_strategy.reward_space,
        )
        if name is not None:
            self.name = name

        match death_strategy:
            case "end":
                self.death_strategy = DeathStrategy.END
            case "respawn":
                self.death_strategy = DeathStrategy.RESPAWN
            case other:
                raise ValueError(f"Unknown death strategy: {other}")
        self.walkable_lasers = walkable_lasers
        self.n_agents = world.n_agents

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

    @property
    def n_arrived(self):
        return self.reward_strategy.n_arrived

    @property
    def done(self):
        return self.n_arrived == self.n_agents or self.reward_strategy.n_deads > 0

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
        # Beware to compute the reward before checking if the episode is done !
        reward = self.reward_strategy.compute_reward(events)
        return Step(
            self.get_observation(),
            self.get_state(),
            reward=reward,
            done=self.done,
            info={"gems_collected": self.world.gems_collected, "exit_rate": self.n_arrived / self.n_agents},
        )

    def reset(self):
        self.world.reset()
        self.reward_strategy.reset()
        return self.get_observation(), self.get_state()

    def get_state(self):
        return State(self.state_generator.get_state())

    def set_state(self, state: State[npt.NDArray[np.float32]] | WorldState):
        if isinstance(state, WorldState):
            world_state = state
        else:
            world_state = self.state_generator.to_world_state(state.data)
        self.reward_strategy.reset()
        events = self.world.set_state(world_state)
        self.reward_strategy.compute_reward(events)

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
