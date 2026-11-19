"""Reward strategies for the `LLE` environment.

The environment can expose a single scalar reward, a multi-objective reward
vector, or a potential-based shaping wrapper around either strategy.
"""

from abc import ABC, abstractmethod
from dataclasses import dataclass
from functools import cached_property
from typing import Callable, Collection, Iterable, Sequence

import numpy as np
import numpy.typing as npt
from marlenv.models import ContinuousSpace

from lle import tiles
from lle.types import Position

from ..world import EventType, World, WorldEvent
from .utils import get_lasers_of

REWARD_GEM = 1.0
REWARD_EXIT = 1.0
REWARD_DONE = 1.0
REWARD_DEATH = -1.0


@dataclass
class RewardStrategy(ABC):
    n_agents: int
    objectives: list[str]

    def __post_init__(self):
        self.reward_space = ContinuousSpace.from_shape(len(self.objectives))
        self.n_arrived = 0
        self.n_deads = 0

    @cached_property
    def n_objectives(self) -> int:
        return self.reward_space.shape[0]

    def reset(self):
        self.n_arrived = 0
        self.n_deads = 0

    @abstractmethod
    def compute_reward(self, events: Sequence[WorldEvent]) -> npt.NDArray[np.float32]:
        """Compute the reward for the given events."""


@dataclass
class SingleObjective(RewardStrategy):
    """Return one scalar reward combining gems, exits, and deaths."""

    def __init__(self, n_agents: int):
        super().__init__(n_agents, ["reward"])

    def compute_reward(self, events: Sequence[WorldEvent]):
        reward = 0.0
        death_reward = 0.0
        for event in events:
            match event.event_type:
                case EventType.AGENT_DIED:
                    reward += REWARD_DEATH
                    self.n_deads += 1
                case EventType.GEM_COLLECTED:
                    reward += REWARD_GEM
                case EventType.AGENT_EXIT:
                    reward += REWARD_EXIT
                    self.n_arrived += 1
        if death_reward != 0:
            reward = death_reward
        elif self.n_arrived == self.n_agents:
            reward += REWARD_DONE
        return np.array([reward], dtype=np.float32)


@dataclass
class MultiObjective(RewardStrategy):
    """Return a reward vector with separate gem, exit, death, and done terms."""

    RW_GEM_IDX = 0
    RW_EXIT_IDX = 1
    RW_DEATH_IDX = 2
    RW_DONE_IDX = 3

    def __init__(self, n_agents: int):
        super().__init__(n_agents, ["gem", "exit", "death", "done"])

    def compute_reward(self, events: Sequence[WorldEvent]):
        reward = np.zeros(self.reward_space.shape, dtype=np.float32)
        for event in events:
            match event.event_type:
                case EventType.AGENT_DIED:
                    reward[MultiObjective.RW_DEATH_IDX] += REWARD_DEATH
                    self.n_deads += 1
                case EventType.GEM_COLLECTED:
                    reward[MultiObjective.RW_GEM_IDX] += REWARD_GEM
                case EventType.AGENT_EXIT:
                    reward[MultiObjective.RW_EXIT_IDX] += REWARD_EXIT
                    self.n_arrived += 1
        # If an agent died, all other rewards are set to 0
        if reward[MultiObjective.RW_DEATH_IDX] != 0:
            death_reward = reward[MultiObjective.RW_DEATH_IDX]
            reward.fill(0.0)
            reward[MultiObjective.RW_DEATH_IDX] = death_reward
        elif self.n_arrived == self.n_agents:
            reward[MultiObjective.RW_DONE_IDX] += REWARD_DONE
        return reward


@dataclass
class PotentialShaped(RewardStrategy):
    """Wrap another strategy with potential-based reward shaping.

    The shaping term rewards agents for reaching the given sets of cells. This follows
    potential-based shaping as described by Ng, Harada, and Russell. The potential is the
    number of (agent, set of cells) pairs such that the agent has not yet reached any cell of the
    set since the last reset, times `reward_value`.

    It does not depend on the kind of world: the positions of the agents are read by
    `get_agents_positions`, which calls the `agents_positions` callback. A subclass may override
    `get_agents_positions` instead of giving a callback. Note that `copy.deepcopy` does not copy a
    function, so a callback that captures a world (such as a lambda) keeps reading that world in
    a copy, and a lambda cannot be pickled.

    Args:
        strategy: the wrapped strategy. With a single objective, the shaping term is added to it;
            with several objectives, the shaping term is an additional objective named `"PBRS"`.
        gamma: the discount factor of the shaping term.
        reward_value: the potential of each (agent, set of cells) pair not reached yet.
        pos_to_reward: the sets of cells to reward.
        agents_positions: returns the current position of each agent. It may only be omitted by
            a subclass that overrides `get_agents_positions`.

    @ai-generated
    """

    gamma: float
    reward_value: float
    strategy: RewardStrategy
    pos_to_reward: list[set[Position]]

    def __init__(
        self,
        strategy: RewardStrategy,
        gamma: float,
        reward_value: float,
        pos_to_reward: Sequence[Collection[Position]],
        agents_positions: Callable[[], Sequence[Position]] | None = None,
    ):
        """
        See the class documentation for the arguments. The objectives are those of `strategy`,
        plus `"PBRS"` when it has several.

        @ai-generated
        """
        self._agents_positions = agents_positions
        self.reward_value = reward_value
        self.gamma = gamma
        objectives = list(strategy.objectives)
        if len(objectives) > 1:
            objectives.append("PBRS")
        super().__init__(strategy.n_agents, objectives)
        self.strategy = strategy
        self.pos_to_reward = [set(cells) for cells in pos_to_reward]
        self._agents_pos_reached = np.full((strategy.n_agents, len(self.pos_to_reward)), False, dtype=np.bool)
        self._previous_potential = self.compute_potential()

    def compute_reward(self, events: Sequence[WorldEvent]):
        reward = self.strategy.compute_reward(events)
        current_potential = self.compute_potential()
        potential_reward = self.gamma * self._previous_potential - current_potential
        if self.n_objectives == 1:
            reward[0] += potential_reward
        else:
            reward = np.concat((reward, [potential_reward]))
        # Book keeping
        self._previous_potential = current_potential
        self.n_deads = self.strategy.n_deads
        self.n_arrived = self.strategy.n_arrived
        return reward

    def get_agents_positions(self) -> Sequence[Position]:
        """
        The current position of each agent, as returned by the `agents_positions` callback.

        Raises a `NotImplementedError` when no callback was given and a subclass does not
        override this method.

        @ai-generated
        """
        if self._agents_positions is None:
            raise NotImplementedError("Give an `agents_positions` callback or override `get_agents_positions`.")
        return self._agents_positions()

    def compute_potential(self):
        """
        Mark the groups of cells that each agent currently stands on as reached, and return the
        potential: `reward_value` times the number of (agent, group) pairs not reached yet.

        @ai-edited
        """
        for agent_num, agent_pos in enumerate(self.get_agents_positions()):
            for j, rewarded_positions in enumerate(self.pos_to_reward):
                if agent_pos in rewarded_positions:
                    self._agents_pos_reached[agent_num, j] = True
        return float(self._agents_pos_reached.size - self._agents_pos_reached.sum()) * self.reward_value

    def reset(self):
        super().reset()
        self.strategy.reset()
        self._agents_pos_reached.fill(False)
        self._previous_potential = self.compute_potential()


@dataclass
class PotentialShapedLLE(PotentialShaped):
    """Potential-based reward shaping that rewards agents for entering the given lasers.

    The cells to reward are the beam cells of each laser source of `lasers_to_reward`, and the
    positions of the agents are those of `world`.
    """

    def __init__(
        self,
        strategy: RewardStrategy,
        world: World,
        gamma: float,
        reward_value: float,
        lasers_to_reward: Iterable[tiles.LaserSource],
    ):
        """
        Shape `strategy` with the beam cells of `lasers_to_reward`, read from `world`, whose
        agents' positions are read at every step.
        """
        self._world = world
        super().__init__(
            strategy,
            gamma,
            reward_value,
            self._compute_positions_to_reward(world, lasers_to_reward),
        )

    def get_agents_positions(self):
        return self._world.agents_positions

    @staticmethod
    def _compute_positions_to_reward(world: World, lasers_to_reward: Iterable[tiles.LaserSource]):
        pos_to_reward = list[set[Position]]()
        for source in lasers_to_reward:
            in_laser_rewards = {laser.pos for laser in get_lasers_of(world, source)}
            pos_to_reward.append(in_laser_rewards)
        return pos_to_reward
