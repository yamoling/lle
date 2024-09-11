from dataclasses import dataclass
from typing import Literal, Optional
from typing_extensions import deprecated
from lle import World, ObservationType


@dataclass
class Builder:
    _world: World
    _obs_type: ObservationType
    _state_type: ObservationType
    _death_strategy: Literal["respawn", "end", "stay"]
    _walkable_lasers: bool

    def __init__(self, world: World):
        self._world = world
        self._obs_type = ObservationType.LAYERED
        self._state_type = ObservationType.STATE
        self._death_strategy = "end"
        self._walkable_lasers = True

    def str_to_obs(self, obs_type: str) -> ObservationType:
        match obs_type:
            case "layered":
                return ObservationType.LAYERED
            case "flattened":
                return ObservationType.FLATTENED
            case "partial3x3":
                return ObservationType.PARTIAL_3x3
            case "partial5x5":
                return ObservationType.PARTIAL_5x5
            case "partial7x7":
                return ObservationType.PARTIAL_7x7
            case "state":
                return ObservationType.STATE
            case "image":
                return ObservationType.RGB_IMAGE
            case "perspective":
                return ObservationType.AGENT0_PERSPECTIVE_LAYERED
        raise ValueError(f"Invalid observation type: {obs_type}")

    def obs_type(
        self,
        obs_type: Literal["layered", "flattened", "partial3x3", "partial5x5", "partial7x7", "state", "image", "perspective"]
        | ObservationType,
    ):
        """
        Set the observation type of the environment (set to ObservationType.LAYERED by default).
        """
        if isinstance(obs_type, str):
            obs_type = self.str_to_obs(obs_type)
        self._obs_type = obs_type
        return self

    def state_type(
        self,
        state_type: Literal["layered", "flattened", "partial3x3", "partial5x5", "partial7x7", "state", "image", "perspective"]
        | ObservationType,
    ):
        """
        Set the state type of the environment (set to ObservationType.STATE by default).
        """
        if isinstance(state_type, str):
            state_type = self.str_to_obs(state_type)
        self._state_type = state_type
        return self

    def walkable_lasers(self, walkable_lasers: bool):
        """
        Set wheter the agents can walk on active laser of different color
        Agent can still die if standing on a disabled laser tile that is reactivated
        """
        self._walkable_lasers = walkable_lasers
        return self

    def death_strategy(self, death_strategy: Literal["respawn", "end", "stay"]):
        """Set the behaviour of the agents when they die (end the episode by default)."""
        self._death_strategy = death_strategy
        return self

    def name(self, name: str):
        """Set the name of the environment."""
        self._env_name = name
        return self

    def single_objective(self):
        # These imports are necessary here to avoid circular imports
        from .single_objective import SOLLE

        return SOLLE(self.core())

    def multi_objective(self):
        # This import is necessary here to avoid circular imports
        from .multi_objective import MOLLE

        return MOLLE(self.core())

    def core(self):
        # This import is necessary here to avoid circular imports
        from .core import Core

        return Core(
            world=self._world,
            obs_type=self._obs_type,
            state_type=self._state_type,
            death_strategy=self._death_strategy,
            walkable_lasers=self._walkable_lasers,
        )

    @deprecated("Use single_objective() or multi_objective() instead.")
    def build(self):
        return self.single_objective()
