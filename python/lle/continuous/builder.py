"""The builder of `CLLE` environments."""

import os
from collections.abc import Callable, Sequence
from typing import Literal

from lle.env.reward_strategy import MultiObjective, RewardStrategy, SingleObjective

from .env import CLLE
from .observations import ObservationType, ObservationTypeLiteral
from .reward_strategy import PotentialShapedCLLE
from .world import ContinuousWorld, PhysicsParams

WorldLoader = Callable[[PhysicsParams | None], ContinuousWorld]


class Builder:
    """Configure and build a `CLLE` environment.

    Start from `lle.continuous.level(...)`, `lle.continuous.from_str(...)` or
    `lle.continuous.from_file(...)`, chain the builder methods, then call `build()`. The map is
    loaded (and checked) right away, but the physics parameters are only checked by `build()`,
    which raises a `ValueError` if they are invalid.

    ```python
    import lle.continuous as clle
    env = clle.level(6).action_repeat(10).max_speed(1.5).pbrs().build()
    ```

    @ai-generated
    """

    def __init__(self, load: WorldLoader, name: str = "CLLE"):
        """
        Args:
            load: builds the world with the given physics parameters (the default ones if
                `None`).
            name: the default name of the environment.

        @ai-generated
        """
        # Load the map once to report an invalid map right away; the world is built again, with
        # the final parameters, by `build()`.
        load(None)
        self._load = load
        self._env_name = name
        self._params = dict[str, float | str]()
        self._action_repeat = 10
        self._obs_type = ObservationType.STATE
        self._state_type = ObservationType.STATE
        self._death_strategy: Literal["end", "respawn"] = "end"
        self._multi_objective = False
        self._pbrs: tuple[float, float, list[int] | None] | None = None
        self._layered_resolution: int = 4
        self._lidar: tuple[int, float] = (16, 5.0)
        self._static_features = False
        self._randomize_lasers = False

    @staticmethod
    def from_str(map_str: str):
        """A builder for the map `map_str` (v1 or TOML), named `CLLE` by default."""
        return Builder(lambda params: ContinuousWorld(map_str, params))

    @staticmethod
    def from_file(path: str | os.PathLike[str]):
        """A builder for the map in the file at `path`, named `CLLE-{basename}` by default."""
        return Builder(lambda params: ContinuousWorld.from_file(path, params), f"CLLE-{os.path.basename(path)}")

    @staticmethod
    def level(n: int):
        """A builder for the built-in level `n` (1 to 6), named `CLLE-lvl{n}` by default."""
        return Builder(lambda params: ContinuousWorld.level(n, params), f"CLLE-lvl{n}")

    def action_repeat(self, k: int):
        """Set the number of ticks of the world per step of the environment (10 by default)."""
        self._action_repeat = k
        return self

    def dt(self, x: float):
        """Set the duration of a tick, in seconds."""
        self._params["dt"] = x
        return self

    def agent_radius(self, r: float):
        """Set the radius of the agents, in cells."""
        self._params["agent_radius"] = r
        return self

    def max_speed(self, v: float):
        """Set the maximal speed of the agents, in cells per second."""
        self._params["max_speed"] = v
        return self

    def action_model(self, model: Literal["velocity", "acceleration"]):
        """Set how actions are interpreted: as velocities (default) or accelerations."""
        self._params["action_model"] = model
        return self

    def max_acceleration(self, a: float):
        """Set the maximal acceleration of the `"acceleration"` action model."""
        self._params["max_acceleration"] = a
        return self

    def damping(self, d: float):
        """Set the velocity damping of the `"acceleration"` action model, in `[0, 1]`."""
        self._params["damping"] = d
        return self

    def gem_radius(self, r: float):
        """Set the radius of the gems, in cells."""
        self._params["gem_radius"] = r
        return self

    def beam_width(self, w: float):
        """Set the width of the laser beams, in cells."""
        self._params["beam_width"] = w
        return self

    def obs_type(self, obs_type: ObservationTypeLiteral | ObservationType):
        """
        Set the observation type (`ObservationType.STATE` by default).

        Raises a `ValueError` right away if `obs_type` is not the name of an observation type.
        """
        self._obs_type = ObservationType.from_str(obs_type) if isinstance(obs_type, str) else obs_type
        return self

    def state_type(self, state_type: ObservationTypeLiteral | ObservationType):
        """
        Set the state type (`ObservationType.STATE` by default). `LIDAR` is a per-agent
        observation, so `build()` refuses it.

        Raises a `ValueError` right away if `state_type` is not the name of an observation type.
        """
        self._state_type = ObservationType.from_str(state_type) if isinstance(state_type, str) else state_type
        return self

    def layered_resolution(self, res: int):
        """Set the number of pixels per cell, along each axis, of the `LAYERED` observation (4 by default)."""
        self._layered_resolution = res
        return self

    def lidar(self, n_rays: int = 16, range: float = 5.0):
        """Set the number of rays of the `LIDAR` observation and their range, in cells."""
        self._lidar = (n_rays, range)
        return self

    def include_static_features(self):
        """
        Append the static features (the centres of the exits, of the gems and of the laser
        sources, with the direction of the sources) to the `STATE` observation and state.
        """
        self._static_features = True
        return self

    def randomize_lasers(self):
        """
        Draw the colour of each laser source at every reset, uniformly among its
        `ContinuousWorld.valid_laser_colours` (whose beam crosses no possible start of an agent of
        another colour). The draws use the `random` module, seeded by `reset(seed=...)` and
        `seed(...)`.
        """
        self._randomize_lasers = True
        return self

    def death_strategy(self, death_strategy: Literal["end", "respawn"]):
        """Set the behaviour of the agents when they die (end the episode by default)."""
        self._death_strategy = death_strategy
        return self

    def multi_objective(self):
        """Use a multi-objective reward (gem, exit, death and done objectives)."""
        self._multi_objective = True
        return self

    def pbrs(self, gamma: float = 0.99, reward_value: float = 0.5, lasers_to_reward: Sequence[int] | None = None):
        """
        Add potential-based reward shaping, such that entering the beam of the given lasers (all
        by default) gives a reward. A laser is given by its laser id.

        @ai-generated
        """
        self._pbrs = (gamma, reward_value, None if lasers_to_reward is None else list(lasers_to_reward))
        return self

    def name(self, name: str):
        """Set the name of the environment."""
        self._env_name = name
        return self

    def build(self):
        """
        Build the environment.

        Raises:
            ValueError: if the physics parameters, `action_repeat`, the layered resolution, the
                lidar parameters, the death strategy or a laser id of `pbrs` are invalid, or if
                the state type is `LIDAR`.
            NotImplementedError: if the death strategy is `"respawn"`.

        @ai-generated
        """
        params = PhysicsParams(**self._params)  # type: ignore[arg-type]
        world = self._load(params)
        reward_strategy: RewardStrategy
        if self._multi_objective:
            reward_strategy = MultiObjective(world.n_agents)
        else:
            reward_strategy = SingleObjective(world.n_agents)
        if self._pbrs is not None:
            gamma, reward_value, lasers = self._pbrs
            if lasers is None:
                lasers = [laser_id for (_, _, _, laser_id) in world.laser_sources]
            reward_strategy = PotentialShapedCLLE(reward_strategy, world, gamma, reward_value, lasers)
        return CLLE(
            world,
            action_repeat=self._action_repeat,
            reward_strategy=reward_strategy,
            obs_type=self._obs_type,
            state_type=self._state_type,
            name=self._env_name,
            death_strategy=self._death_strategy,
            layered_resolution=self._layered_resolution,
            lidar_n_rays=self._lidar[0],
            lidar_range=self._lidar[1],
            static_features=self._static_features,
            randomize_lasers=self._randomize_lasers,
        )
