from typing import Literal

from marlenv.wrappers import EnvPool

from lle.generator import generate
from lle.observations import ObservationType, ObservationTypeLiteral

from .env import LLE
from .reward_strategy import SingleObjective


def make_pool(
    size: int,
    *,
    obs_type: ObservationTypeLiteral = "layered",
    state_type: ObservationTypeLiteral = "state",
    height: int = 12,
    width: int = 13,
    n_agents: int = 4,
    n_lasers: int = 3,
    t_max: int = 21,
    n_walls: int | Literal["auto"] = "auto",
    seed: int | None = None,
):
    """
    Create a pool of `size` LLE environments dynamically generated according to the given parameters.
    """
    worlds = generate(
        "level6_style",
        n=size,
        height=height,
        width=width,
        n_agents=n_agents,
        n_lasers=n_lasers,
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
