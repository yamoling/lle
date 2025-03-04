from dataclasses import dataclass
from typing import Literal, Optional, Sequence
from lle import World, ObservationType
from lle import tiles
from ..types import Position
import marlenv
from .reward_strategy import RewardStrategy, SingleObjective, MultiObjective, PotentialShapedLLE
from .env import LLE
from .extras_generators import ExtraGenerator, LaserSubgoal, MultiGenerator, NoExtras


@dataclass
class Builder:
    _world: World
    _obs_type: ObservationType
    _state_type: ObservationType
    _death_strategy: Literal["respawn", "end"]
    _walkable_lasers: bool
    _env_name: str
    _reward_strategy: RewardStrategy
    _extras_generator: ExtraGenerator
    _randomize_lasers: bool

    def __init__(self, world: World):
        self._world = world
        self._obs_type = ObservationType.LAYERED
        self._state_type = ObservationType.STATE
        self._death_strategy = "end"
        self._walkable_lasers = True
        self._env_name = "LLE"
        self._reward_strategy = SingleObjective(world.n_agents)
        self._extras_generator = NoExtras(world.n_agents)
        self._randomize_lasers = False

    def obs_type(
        self,
        obs_type: Literal["layered", "flattened", "partial3x3", "partial5x5", "partial7x7", "state", "image", "perspective"]
        | ObservationType,
    ):
        """
        Set the observation type of the environment (set to ObservationType.LAYERED by default).
        """
        if isinstance(obs_type, str):
            obs_type = str_to_obs(obs_type)
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
            state_type = str_to_obs(state_type)
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

    def multi_objective(self):
        match self._reward_strategy:
            case MultiObjective():
                return self
            case PotentialShapedLLE():
                if isinstance(self._reward_strategy.strategy, MultiObjective):
                    return self
                raise ValueError("Cannot set multi-objective after setting a reward shaping strategy. Call `multi_objective()` first.")
        self._reward_strategy = MultiObjective(self._world.n_agents)
        self._env_name = f"{self._env_name}-MO"
        return self

    def pbrs(
        self,
        gamma: float = 0.99,
        reward_value: float = 0.5,
        lasers_to_reward: Optional[Sequence[tiles.LaserSource | Position]] = None,
        with_extras: bool = True,
    ):
        """
        Add Potential-Based Reward Shaping such that crossing the given lasers (all by default) gives a reward.
        """
        if lasers_to_reward is None:
            sources = self._world.laser_sources
        else:
            sources = []
            for source in lasers_to_reward:
                match source:
                    case tuple() as pos:
                        sources.append(self._world.source_at(pos))
                    case tiles.LaserSource():
                        sources.append(source)
                    case other:
                        raise ValueError(f"Invalid laser source: {other}")
        if with_extras:
            self.add_extras(LaserSubgoal(self._world, sources))
        self._env_name = f"{self._env_name}-PBRS"
        self._reward_strategy = PotentialShapedLLE(
            self._reward_strategy,
            self._world,
            gamma,
            reward_value,
            sources,
        )
        return self

    def randomize_lasers(self):
        """Randomize the colour of the lasers at each reset."""
        self._randomize_lasers = True
        return self

    def add_extras(self, *extras: Literal["laser_subgoal"] | ExtraGenerator):
        """
        Add extra information to the observation.
        """
        if len(extras) == 0:
            return self

        match self._extras_generator:
            case NoExtras():
                all_extras = list[ExtraGenerator]()
            case MultiGenerator():
                all_extras = self._extras_generator.generators
            case ExtraGenerator() as other:
                all_extras = [other]

        for extra_type in extras:
            match extra_type:
                case NoExtras():
                    pass
                case ExtraGenerator():
                    all_extras.append(extra_type)
                case "laser_subgoal":
                    all_extras.append(LaserSubgoal(self._world))
                case other:
                    raise ValueError(f"Invalid extra type: {other}")
        if len(all_extras) == 1:
            self._extras_generator = all_extras[0]
        else:
            self._extras_generator = MultiGenerator(*all_extras)
        return self

    def build(self) -> LLE:
        env = LLE(
            world=self._world,
            obs_type=self._obs_type,
            state_type=self._state_type,
            death_strategy=self._death_strategy,
            walkable_lasers=self._walkable_lasers,
            name=self._env_name,
            reward_strategy=self._reward_strategy,
            extras_generator=self._extras_generator,
            randomize_lasers=self._randomize_lasers,
        )
        return env

    def builder(self):
        env = self.build()
        return marlenv.Builder(env)


def str_to_obs(
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
