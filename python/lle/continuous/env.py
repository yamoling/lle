"""The `CLLE` environment: the `marlenv` interface of a `ContinuousWorld`."""

import math
import random
from dataclasses import dataclass
from functools import cached_property
from typing import Any, Literal

import numpy as np
import numpy.typing as npt
from marlenv.models import ContinuousMARLEnv, ContinuousSpace, Observation, State, Step

from lle.env.common import DeathStrategy
from lle.env.reward_strategy import RewardStrategy, SingleObjective
from lle.world import EventType, WorldEvent

from .observations import ObservationType
from .world import ContinuousWorld, ContinuousWorldState

TILE_SIZE = 32
"""The number of pixels per cell of the rendered frames."""


def _positive_int(value: Any, name: str) -> int:
    """
    `value` as an `int`, or a `ValueError` if it is not a positive integer (booleans excluded).

    @ai-generated
    """
    if isinstance(value, bool) or not isinstance(value, (int, np.integer)) or value < 1:
        raise ValueError(f"{name} must be a positive integer, got {value!r}")
    return int(value)


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

    The observations and the state are computed by the world, according to `obs_type` and
    `state_type` (see `ObservationType`): `STATE` (optionally with the static features),
    `LAYERED` (with `layered_resolution` pixels per cell), `LIDAR` (with `lidar_n_rays` rays of
    length `lidar_range`; observations only) and `RGB_IMAGE`.

    With `randomize_lasers`, every reset draws the colour of each laser source, in laser id
    order, uniformly among the colours that are valid for it (whose beam crosses no possible start
    of an agent of another colour), with the `random` module. Otherwise, every reset restores the
    colours that the sources had when the environment was created.

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
    layered_resolution: int
    lidar_n_rays: int
    lidar_range: float
    static_features: bool
    randomize_lasers: bool

    def __init__(
        self,
        world: ContinuousWorld,
        action_repeat: int = 10,
        reward_strategy: RewardStrategy | None = None,
        obs_type: ObservationType = ObservationType.STATE,
        state_type: ObservationType = ObservationType.STATE,
        name: str | None = None,
        death_strategy: Literal["respawn", "end"] = "end",
        layered_resolution: int = 4,
        lidar_n_rays: int = 16,
        lidar_range: float = 5.0,
        static_features: bool = False,
        randomize_lasers: bool = False,
    ):
        """
        Raises:
            ValueError: if `action_repeat`, `layered_resolution` or `lidar_n_rays` is not a
                positive integer, `lidar_range` is not positive and finite, `death_strategy` is
                unknown, or `state_type` is `LIDAR`.
            NotImplementedError: if `death_strategy` is `"respawn"`.

        @ai-generated
        """
        action_repeat = _positive_int(action_repeat, "action_repeat")
        self.layered_resolution = _positive_int(layered_resolution, "layered_resolution")
        self.lidar_n_rays = _positive_int(lidar_n_rays, "lidar_n_rays")
        if (
            isinstance(lidar_range, bool)
            or not isinstance(lidar_range, (int, float, np.integer, np.floating))
            or not math.isfinite(lidar_range)
            or lidar_range <= 0
        ):
            raise ValueError(f"lidar_range must be positive and finite, got {lidar_range!r}")
        self.lidar_range = float(lidar_range)
        if ObservationType(state_type) == ObservationType.LIDAR:
            raise ValueError("The state type cannot be LIDAR, a per-agent observation: use STATE, LAYERED or RGB_IMAGE")
        self.static_features = bool(static_features)
        self.randomize_lasers = bool(randomize_lasers)
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
        self._initial_laser_colours = [(laser_id, colour) for (_, _, colour, laser_id) in world.laser_sources]
        self._laser_colour_choices = self._valid_laser_colours(world)
        super().__init__(
            world.n_agents,
            action_space=ContinuousSpace.from_shape((world.n_agents, 2), low=-1.0, high=1.0),
            observation_shape=self._shape(ObservationType(obs_type)),
            state_shape=self._shape(ObservationType(state_type)),
            reward_space=self.reward_strategy.reward_space,
        )
        self._name = name
        self.done = False

    @staticmethod
    def _valid_laser_colours(world: ContinuousWorld):
        """
        For each laser source, in laser id order, its laser id and the colours it may take: those
        whose beam covers no possible start of an agent of another colour.

        @ai-generated
        """
        choices = list[tuple[int, list[int]]]()
        sources = sorted(zip(world.laser_sources, world.beam_cells), key=lambda source: source[0][3])
        for (_, _, _, laser_id), beam_cells in sources:
            cells = set(beam_cells)
            valid = [
                colour
                for colour in range(world.n_colours)
                if all(
                    agent_colour == colour or cells.isdisjoint(starts)
                    for agent_colour, starts in zip(world.agents_colours, world.possible_starts)
                )
            ]
            choices.append((laser_id, valid))
        return choices

    def _shape(self, obs_type: ObservationType) -> tuple[int, ...]:
        """
        The shape of one observation (or of the state) of type `obs_type`.

        @ai-generated
        """
        match obs_type:
            case ObservationType.STATE:
                return (self._world.observation_size_for(self.static_features),)
            case ObservationType.LAYERED:
                return self._world.layered_shape(self.layered_resolution)
            case ObservationType.LIDAR:
                return (self._world.lidar_size(self.lidar_n_rays),)
            case ObservationType.RGB_IMAGE:
                return (self._world.height * TILE_SIZE, self._world.width * TILE_SIZE, 3)

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
        if self.randomize_lasers:
            for laser_id, colours in self._laser_colour_choices:
                if len(colours) > 0:
                    self._world.set_laser_colour(laser_id, random.choice(colours))
        else:
            self._restore_laser_colours()
        self.reward_strategy.reset()
        self.done = False
        return self.get_observation(), self.get_state()

    def _restore_laser_colours(self):
        """
        Give back to each laser source the colour it had when the environment was created.

        @ai-generated
        """
        current = {laser_id: colour for (_, _, colour, laser_id) in self._world.laser_sources}
        for laser_id, colour in self._initial_laser_colours:
            if current[laser_id] != colour:
                self._world.set_laser_colour(laser_id, colour)

    def seed(self, seed_value: int):
        random.seed(seed_value)
        self._world.seed(seed_value)

    def get_observation(self):
        """
        The observation of every agent, of shape `(n_agents, *observation_shape)` and dtype
        `float32`, without extras.

        @ai-generated
        """
        world = self._world
        match ObservationType[self.obs_type]:
            case ObservationType.STATE:
                data = np.stack([world.observation(i, self.static_features) for i in range(self.n_agents)])
            case ObservationType.LAYERED:
                data = np.tile(world.layered(self.layered_resolution), (self.n_agents, 1, 1, 1))
            case ObservationType.LIDAR:
                data = np.stack([world.lidar(i, self.lidar_n_rays, self.lidar_range) for i in range(self.n_agents)])
            case ObservationType.RGB_IMAGE:
                data = np.tile(world.get_image(), (self.n_agents, 1, 1, 1)).astype(np.float32)
        return Observation(data, self.available_actions())

    def get_state(self):
        """
        The state of the world, of shape `state_shape` and dtype `float32`.

        @ai-generated
        """
        match ObservationType[self.state_type]:
            case ObservationType.LAYERED:
                data = self._world.layered(self.layered_resolution)
            case ObservationType.RGB_IMAGE:
                data = self._world.get_image().astype(np.float32)
            case _:
                data = self._world.state_vector(self.static_features)
        return State(data)

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
        """The current frame, of shape `(height * 32, width * 32, 3)` and dtype `uint8`."""
        return self._world.get_image()

    def render(self, mode: Literal["human", "rgb_array"] = "human"):
        """
        Render the environment: `"human"` shows the frame in an OpenCV window (as `marlenv`
        does), and `"rgb_array"` returns it (see `get_image`).

        Raises a `ValueError` for any other mode.

        @ai-generated
        """
        match mode:
            case "human":
                return super().render()
            case "rgb_array":
                return self.get_image()
        raise ValueError(f"Unknown render mode {mode!r}: expected 'human' or 'rgb_array'")

    def compute_done(self):
        """
        Whether the episode is over: every agent has arrived, or an agent died.

        @ai-generated
        """
        return all(self._world.agents_arrived) or not all(self._world.agents_alive)
