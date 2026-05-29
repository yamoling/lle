from typing import Literal

from marlenv.catalog import EnvPool

from lle.generator import LooseCooperationSpec, generate
from lle.observations import ObservationType, ObservationTypeLiteral
from lle.solver.cooperation_level import CooperationLevel

from .env import LLE
from .reward_strategy import SingleObjective


def make_pool(
    size: int,
    *,
    obs_type: ObservationTypeLiteral = "layered",
    state_type: ObservationTypeLiteral = "state",
    cooperation: LooseCooperationSpec | None = ("at-least", CooperationLevel.COOPERATIVE),
    height: int = 12,
    width: int = 13,
    n_agents: int = 4,
    n_lasers: int = 3,
    t_max: int = 21,
    n_walls: int | Literal["auto"] = "auto",
    seed: int | None = None,
):
    """Create an environment pool backed by freshly generated worlds.

    The helper samples `size` worlds with the Level-6-style generator and wraps
    them in a `marlenv` `EnvPool`.
    """
    worlds = generate(
        "level6_style",
        n=size,
        height=height,
        width=width,
        n_agents=n_agents,
        n_lasers=n_lasers,
        cooperation=cooperation,
        t_max=t_max,
        n_walls=n_walls,
        seed=seed,
        n_jobs="auto",
    )
    reward_strategy = SingleObjective(n_agents)
    envs = [
        LLE(
            w,
            reward_strategy,
            ObservationType.from_str(obs_type),
            ObservationType.from_str(state_type),
        )
        for w in worlds
    ]
    return EnvPool(envs)
