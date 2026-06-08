from __future__ import annotations

from typing import Literal

from ._args import GenerateArgs
from .constructive import ConstructiveGenerator
from .level6_style import Level6StyleGenerator
from .random import RandomGenerator


def generate(
    kind: Literal["random", "constructive", "level6_style"] = "level6_style",
    *,
    n: int = 1,
    height: int | None = None,
    width: int | None = None,
    n_agents: int | None = None,
    n_lasers: int | Literal["auto"] = "auto",
    cooperation: bool | None = None,
    t_min: int = 0,
    t_max: int | Literal["auto"] = "auto",
    n_walls: int | Literal["auto"] = "auto",
    seed: int | None = None,
    max_attempts: int | None = None,
    n_jobs: int | Literal["auto"] = "auto",
    quiet: bool = False,
):
    """
    Build a solvable `World` on demand using a SAT-verified procedural generator.

    # Parameters:
    - `kind`: the kind of generator to use. Refer to generator classes in the `lle.generator` module for more information.
    - `cooperation`: `None`: any cooperation level is accepted, including non-cooperative maps; `True` for any cooperative level, `False` for no cooperation;
    - `n_walls`: the number of walls to place. If `"auto"`, 10% of the grid is filled with walls.
    - `t_min`: a guaranteed lower bound on the solution length. The generator only accepts
      levels that are *not* solvable in fewer than `t_min` steps (and still solvable within
      `t_max`). If `"auto"`, defaults to `0` (no lower bound). Must satisfy `0 <= t_min <= t_max`.
    - `t_max`: the maximal solution path length. If `"auto"`, defaults to `width * height // 2`.
    - `n`: the number of worlds to generate.
    - `max_attempts`: the maximum number of attempts to generate valid worlds before stopping. If `None`, there is no limit.
    - `n`: the number of worlds to generate
    - `n_jobs`: the number jobs to run in parallel. When `auto`, spawns `n_cpus - 1` jobs if `n` > 1 or spawns 1 job is `n` = 1.
    - `quiet`: whether to remove the progress bar when `n` > 1.

    # Returns:
        - When `max_attempts` is not provided (default):
            - A single `World` if `n=1`;
            - A Generator of `n` `World` if `n` > 1.
        - When `max_attempts` is set:
            - A `World | None` for `n` = 1;
            - A Generator of at most `n` worlds `World`s for `n` > 1.

    Raises:
    -------
        - `ValueError` if arguments are invalid;
        - `ValueError` if there is no default argument value for the given combination.

    # Examples:
    ```python
    import lle
    world = lle.generate("level6_style", n_agents=4, n_lasers=3)
    world = lle.generate("random", n=10, width=5, height=7, n_agents=2, seed=0)
    world = lle.generate("random", cooperation=("at-least", "mutual"))
    world_or_none = lle.genrate("random", max_attempts=10, cooperation="mutual")
    ```
    """
    args = GenerateArgs(
        kind=kind,
        height=height,
        width=width,
        n_agents=n_agents,
        n_lasers=n_lasers,
        cooperation=cooperation,
        t_max=t_max,
        t_min=t_min,
        n_walls=n_walls,
        max_attempts=max_attempts,
        n_jobs=n_jobs,
    ).resolve(n)
    if kind == "random":
        generator = RandomGenerator(
            height=args.height,
            width=args.width,
            n_agents=args.n_agents,
            n_lasers=args.n_lasers,
            cooperation=args.cooperation,
            n_walls=args.n_walls,
            t_max=args.t_max,
            t_min=args.t_min,
        )
    elif kind == "constructive":
        generator = ConstructiveGenerator(
            height=args.height,
            width=args.width,
            n_agents=args.n_agents,
            n_lasers=args.n_lasers,
            cooperation=args.cooperation,
            n_walls=args.n_walls,
            t_max=args.t_max,
            t_min=args.t_min,
        )
    elif kind == "level6_style":
        generator = Level6StyleGenerator(
            height=args.height,
            width=args.width,
            n_agents=args.n_agents,
            n_lasers=args.n_lasers,
            n_walls=args.n_walls,
            t_max=args.t_max,
            t_min=args.t_min,
            cooperation=args.cooperation,
        )
    else:
        raise ValueError(f"Unknown kind: {kind!r}. Expected 'random', 'constructive', or 'level6_style'.")
    if args.n_jobs > 1:
        worlds = generator.generate_n(n, args.n_jobs, seed, args.max_attempts, quiet=quiet)
        if n == 1:
            return next(worlds)
        return worlds
    return generator.generate(seed=seed, max_attempts=max_attempts)
