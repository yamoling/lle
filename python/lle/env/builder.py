from dataclasses import dataclass
from typing import Literal
from lle import World, ObservationType
from .reward_strategy import RewardStrategy, SingleObjective, MultiObjective


@dataclass
class Builder:
    _world: World
    _obs_type: ObservationType
    _state_type: ObservationType
    _death_strategy: Literal["respawn", "end"]
    _walkable_lasers: bool
    _env_name: str
    _reward_strategy: RewardStrategy

    def __init__(self, world: World):
        self._world = world
        self._obs_type = ObservationType.LAYERED
        self._state_type = ObservationType.STATE
        self._death_strategy = "end"
        self._walkable_lasers = True
        self._env_name = "LLE"
        self._reward_strategy = SingleObjective(world.n_agents)

    def str_to_obs(
        self,
        obs_type: Literal[
            "layered",
            "flattened",
            "partial3x3",
            "partial5x5",
            "partial7x7",
            "state",
            "image",
            "perspective",
            "normalized-state",
        ],
    ) -> ObservationType:
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
            case "state-normalized":
                return ObservationType.NORMALIZED_STATE
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

    def death_strategy(self, death_strategy: Literal["respawn", "end"]):
        """Set the behaviour of the agents when they die (end the episode by default)."""
        self._death_strategy = death_strategy
        return self

    def name(self, name: str):
        """Set the name of the environment."""
        self._env_name = name
        return self

    def multi_objective(self, is_multi_objective: bool = True):
        if not is_multi_objective:
            return self.single_objective()
        self._reward_strategy = MultiObjective(self._world.n_agents)
        self._env_name = f"{self._env_name}-MO"
        return self

    def single_objective(self, is_single_objective: bool = True):
        if not is_single_objective:
            return self.multi_objective()
        self._reward_strategy = SingleObjective(self._world.n_agents)
        self._env_name = f"{self._env_name}-SO"
        return self

    def build(self):
        # avoid circular imports
        from .env import LLE

        return LLE(
            world=self._world,
            obs_type=self._obs_type,
            state_type=self._state_type,
            death_strategy=self._death_strategy,
            walkable_lasers=self._walkable_lasers,
            name=self._env_name,
            reward_strategy=self._reward_strategy,
        )
