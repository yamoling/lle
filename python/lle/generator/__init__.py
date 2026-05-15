"""Public generator API: lle.generate.

Different kinds expose different parameter sets via typing.overload for
static type checkers; the runtime dispatcher accepts the union of kwargs
and delegates to the corresponding private generator class.
"""

from __future__ import annotations

from typing import Literal, overload

from lle import World

from ._constructive import _ConstructiveGenerator
from ._level6_style import _Level6StyleGenerator
from ._random import _RandomGenerator


@overload
def generate(
    kind: Literal["random", "constructive"],
    *,
    height: int = 5,
    width: int = 5,
    n_agents: int = 2,
    n_lasers: int | None = None,
    cooperative: bool = False,
    n_walls: int | Literal["auto"] = "auto",
    t_max: int | None = None,
    seed: int | None = None,
    n: Literal[1] = 1,
    n_jobs: int | Literal["auto"] = 1,
) -> World: ...


@overload
def try_generate(
    kind: Literal["random", "constructive"],
    *,
    max_attempts: int = 10_000,
    height: int = 5,
    width: int = 5,
    n_agents: int = 2,
    n_lasers: int | None = None,
    cooperative: bool = False,
    n_walls: int | Literal["auto"] = "auto",
    t_max: int | None = None,
    seed: int | None = None,
) -> World | None: ...


@overload
def generate(
    kind: Literal["random", "constructive"],
    *,
    height: int = 5,
    width: int = 5,
    n_agents: int = 2,
    n_lasers: int | None = None,
    cooperative: bool = False,
    n_walls: int | Literal["auto"] = "auto",
    t_max: int | None = None,
    seed: int | None = None,
    n: int,
    n_jobs: int | Literal["auto"] = "auto",
) -> list[World]: ...


@overload
def generate(
    kind: Literal["level6_style"] = "level6_style",
    *,
    height: int = 12,
    width: int = 13,
    n_agents: int = 4,
    n_lasers: int = 3,
    t_max: int = 21,
    n_walls: int | Literal["auto"] = "auto",
    seed: int | None = None,
    n: Literal[1] = 1,
    n_jobs: int | Literal["auto"] = 1,
) -> World: ...


@overload
def generate(
    kind: Literal["level6_style"] = "level6_style",
    *,
    height: int = 12,
    width: int = 13,
    n_agents: int = 4,
    n_lasers: int = 3,
    t_max: int = 21,
    n_walls: int | Literal["auto"] = "auto",
    seed: int | None = None,
    n: int,
    n_jobs: int | Literal["auto"] = "auto",
) -> list[World]: ...


@overload
def try_generate(
    kind: Literal["level6_style"],
    *,
    max_attempts: int,
    height: int = 12,
    width: int = 13,
    n_agents: int = 4,
    n_lasers: int = 3,
    t_max: int = 21,
    n_walls: int | Literal["auto"] = "auto",
    seed: int | None = None,
) -> World | None: ...


def _resolve_lasers(lasers: int | None, agents: int, cooperative: bool) -> int:
    if lasers is not None:
        return lasers
    if cooperative:
        return max(1, agents - 1)
    return 0


def _no_default_for(kind: str, arg_name: str):
    return ValueError(f"No default value for parameter {arg_name} with kind={kind!r}")


def generate(
    kind: Literal["random", "constructive", "level6_style"] = "level6_style",
    *,
    height: int | None = None,
    width: int | None = None,
    n_agents: int | None = None,
    n_lasers: int | None = None,
    cooperative: bool | None = None,
    t_max: int | None = None,
    n_walls: int | Literal["auto"] = "auto",
    seed: int | None = None,
    max_attempts: int | None = None,
    n_jobs: int | Literal["auto"] | None = None,
    n: int = 1,
):
    """
    Build a solvable `World` on demand using a SAT-verified procedural generator.

    Raises:
        - `ValueError` if `kind` is unknown or arguments are invalid;
        - `ValueError` if `cooperative` is passed to `kind="level6_style"`;


    Parameters
    ----------
    - `kind`: the kind of generator to use. Refer to generator calsses in the `lle.generator` module for more information.
    - `cooperative`: whether the generated world must enforce coopeartion or not (i.e. an agent must block a laser for an other agent).
    - `n_walls`: the number of walls to place. If `auto`, 10% of the area is filled with walls.
    - `t_max`: the maximal solution path length.
    - `n`: the number of worlds to generate.
    - `n_jobs`: the number of parallel jobs to run. If `auto`, spawns `n_cpus - 1` jobs.

    Example:
    -------
    ```python
    import lle
    world = lle.generate("level6_style", n_agents=4, n_lasers=3)
    world = lle.generate("random", n=10, width=5, height=7, n_agents=2, seed=0)
    ```
    """
    # Handle default argument by generator kind
    if (width is None and height is not None) or (height is None and width is not None):
        raise ValueError("Cannot infer the size of the grid: either provide `width` and `heights`, or none of them.")
    if width is None or height is None:
        if kind in ("random", "constructive"):
            height, width = (5, 5)
        elif kind == "level6_style":
            height, width = (12, 13)
        else:
            raise _no_default_for(kind, "size")
    # assert width is not None and height is not None
    if n_agents is None:
        if kind in ("random", "constructive"):
            n_agents = 2
        elif kind == "level6_style":
            n_agents = 4
        else:
            raise _no_default_for(kind, "n_agents")
    if cooperative is None:
        if kind in ("random", "constructive"):
            cooperative = False
        elif kind == "level6_style":
            cooperative = True
        else:
            raise _no_default_for(kind, "cooperative")
    if n_lasers is None:
        if kind == "level6_style":
            n_lasers = 3
        else:
            n_lasers = _resolve_lasers(n_lasers, n_agents, cooperative)
    if kind == "level6_style":
        if not cooperative:
            raise ValueError("Levels in the style of level 6 must be cooperative.")
        if t_max is None:
            t_max = 21
    if n_walls == "auto":
        n_walls = width * height // 10
    if kind == "random":
        generator = _RandomGenerator(
            height=height,
            width=width,
            n_agents=n_agents,
            n_lasers=n_lasers,
            cooperative=cooperative,
            n_walls=n_walls,
            t_max=t_max,
        )
    elif kind == "constructive":
        generator = _ConstructiveGenerator(
            height=height,
            width=width,
            n_agents=n_agents,
            n_lasers=n_lasers,
            cooperative=cooperative,
            n_walls=n_walls,
            t_max=t_max,
        )
    elif kind == "level6_style":
        generator = _Level6StyleGenerator(
            height=height,
            width=width,
            n_agents=n_agents,
            n_lasers=n_lasers,
            n_walls=n_walls,
            t_max=t_max,
        )
    else:
        raise ValueError(f"Unknown kind: {kind!r}. Expected 'random', 'constructive', or 'level6_style'.")
    # According to the @overloads,
    #   - if n == 1, default value is 1
    #   - if n > 1, default value if "auto"
    if n_jobs is None:
        if n > 1:
            n_jobs = "auto"
        else:
            n_jobs = 1
    if n_jobs == "auto":
        from multiprocessing import cpu_count

        n_jobs = cpu_count() - 1
    if n_jobs > 1:
        worlds = generator.generate_n(n, n_jobs, seed)
        if n == 1:
            return worlds[0]
        return worlds
    return generator.generate(seed=seed, max_attempts=max_attempts)


def try_generate(*args, **kwargs) -> World | None:
    """See `lle.generate`."""
    return generate(*args, **kwargs)


__all__ = ["generate", "try_generate"]
