"""The `CLLE` environment: the `marlenv` interface of a `ContinuousWorld`."""

import random
from dataclasses import dataclass
from functools import cached_property
from typing import Literal

import numpy as np
import numpy.typing as npt
from marlenv.models import ContinuousMARLEnv, ContinuousSpace, Observation, State, Step

from lle.env.common import DeathStrategy
from lle.env.reward_strategy import RewardStrategy, SingleObjective
from lle.world import EventType, WorldEvent

from .observations import ObservationType
from .world import ContinuousWorld, ContinuousWorldState


@dataclass
class CLLE(ContinuousMARLEnv):
    """A `marlenv` environment backed by a `ContinuousWorld`.

    Build it with `lle.continuous.level(...)`, `lle.continuous.from_str(...)` or
    `lle.continuous.from_file(...)`, then adjust the configuration with the `Builder` methods
    before calling `build()`.

    The joint action has the shape `(n_agents, 2)` with values in `[-1, 1]`: `(ax, ay)` for each
    agent, `ax` towards the east and `ay` towards the south. One step of the environment applies
    the joint action for `action_repeat` ticks of the world (fewer if the episode ends earlier),
    and the reward is computed from the events of all these ticks.

    Example
    -------
    ```python
    import lle.continuous as clle
    env = clle.level(6).action_repeat(10).build()
    obs, state = env.reset()
    step = env.step(env.sample_action())
    ```

    @ai-generated
    """

    obs_type: str
    state_type: str
    death_strategy: DeathStrategy
    action_repeat: int
    reward_strategy: RewardStrategy

    def __init__(
        self,
        world: ContinuousWorld,
        action_repeat: int = 10,
        reward_strategy: RewardStrategy | None = None,
        obs_type: ObservationType = ObservationType.STATE,
        state_type: ObservationType = ObservationType.STATE,
        name: str | None = None,
        death_strategy: Literal["respawn", "end"] = "end",
    ):
        """
        Raises:
            ValueError: if `action_repeat` is not a positive integer or `death_strategy` is
                unknown.
            NotImplementedError: if `death_strategy` is `"respawn"`.

        @ai-generated
        """
        if isinstance(action_repeat, bool) or not isinstance(action_repeat, (int, np.integer)) or action_repeat < 1:
            raise ValueError(f"action_repeat must be a positive integer, got {action_repeat!r}")
        match DeathStrategy.from_str(death_strategy):
            case DeathStrategy.END:
                self.death_strategy = DeathStrategy.END
            case DeathStrategy.RESPAWN:
                raise NotImplementedError("Respawn strategy is not implemented yet")
        self._world = world
        self.action_repeat = int(action_repeat)
        self.obs_type = ObservationType(obs_type).name
        self.state_type = ObservationType(state_type).name
        if reward_strategy is None:
            reward_strategy = SingleObjective(world.n_agents)
        self.reward_strategy = reward_strategy
        super().__init__(
            world.n_agents,
            action_space=ContinuousSpace.from_shape((world.n_agents, 2), low=-1.0, high=1.0),
            observation_shape=(world.observation_size,),
            state_shape=(world.state_size,),
            reward_space=self.reward_strategy.reward_space,
        )
        self._name = name
        self.done = False

    @property
    def name(self):
        if self._name is not None:
            return self._name
        return super().name

    @property
    def world(self):
        return self._world

    @cached_property
    def width(self) -> int:
        return self._world.width

    @cached_property
    def height(self) -> int:
        return self._world.height

    def step(self, action: npt.ArrayLike):
        """
        Apply `action` for `action_repeat` ticks (fewer if the episode ends earlier) and compute
        the reward from all the events of these ticks.

        Raises a `ValueError` if the environment is done, and the errors of
        `ContinuousWorld.step` for an invalid action.

        @ai-generated
        """
        if self.done:
            raise ValueError("Cannot step in a done environment")
        actions = np.asarray(action)
        outcome = self._world.step(actions, self.action_repeat)
        # Compute the reward before checking whether the episode is done.
        reward = self.reward_strategy.compute_reward(outcome.events)
        self.done = self.compute_done()
        alive = self._world.agents_alive
        arrived = self._world.agents_arrived
        n_arrived = sum(arrived)
        metrics = {}
        for i in range(self.n_agents):
            metrics[f"has-arrived-{i}"] = arrived[i]
            metrics[f"is-alive-{i}"] = alive[i]
        return Step(
            actions,
            self.get_observation(),
            self.get_state(),
            reward=reward,
            done=self.done,
            info={
                "gems_collected": sum(self._world.gems_collected),
                "exit_rate": n_arrived / self.n_agents,
                "joint_exit": int(n_arrived == self.n_agents),
                "n_ticks": outcome.n_ticks,
                **metrics,
            },
        )

    def reset(self, *, seed: int | None = None):
        """
        Reset the world, the reward strategy and `done`, after seeding the world and `random`
        when `seed` is given.

        @ai-generated
        """
        if seed is not None:
            self.seed(seed)
        self._world.reset()
        self.reward_strategy.reset()
        self.done = False
        return self.get_observation(), self.get_state()

    def seed(self, seed_value: int):
        random.seed(seed_value)
        self._world.seed(seed_value)

    def get_observation(self):
        """
        The `STATE` observation of every agent, stacked into an `(n_agents, observation_size)`
        `float32` array, without extras.

        @ai-generated
        """
        data = np.stack([self._world.observation(i) for i in range(self.n_agents)])
        return Observation(data, self.available_actions())

    def get_state(self):
        return State(self._world.state_vector())

    def set_state(self, state: State | ContinuousWorldState):
        """
        Restore a `ContinuousWorldState`, reset the reward strategy and recompute `done` from the
        flags of the world.

        After the reset, the strategy is fed one `AGENT_EXIT` event per agent that has already
        arrived (the reward is discarded), like `LLE.set_state` feeds the events of the restored
        state. Its arrival count then matches the world, so that the agents who arrive later still
        earn the completion reward, and a shaping wrapper starts from the restored positions.

        Raises:
            NotImplementedError: if `state` is a `State`: its normalized vector cannot be
                inverted exactly.
            InvalidWorldStateError: if the state is invalid for the world.

        @ai-generated
        """
        if not isinstance(state, ContinuousWorldState):
            raise NotImplementedError("CLLE.set_state only accepts a ContinuousWorldState: a normalized State vector cannot be inverted.")
        self._world.set_state(state)
        self.reward_strategy.reset()
        arrivals = [WorldEvent(EventType.AGENT_EXIT, agent_id=i) for i, arrived in enumerate(self._world.agents_arrived) if arrived]
        self.reward_strategy.compute_reward(arrivals)
        self.done = self.compute_done()

    def get_image(self):
        raise NotImplementedError("Rendering is not implemented for CLLE yet")

    def compute_done(self):
        """
        Whether the episode is over: every agent has arrived, or an agent died.

        @ai-generated
        """
        return all(self._world.agents_arrived) or not all(self._world.agents_alive)
