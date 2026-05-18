"""Public generator API: lle.generate.

Different kinds expose different parameter sets via typing.overload for
static type checkers; the runtime dispatcher accepts the union of kwargs
and delegates to the corresponding private generator class.
"""

from __future__ import annotations

from typing import Literal, overload

from ..solver.cooperation_level import CooperationLevel, CooperationLevelStr
from ..world import World
from ._args import _GenerateArgs
from ._base import CooperationSpec
from ._constructive import _ConstructiveGenerator
from ._level6_style import _Level6StyleGenerator
from ._random import _RandomGenerator

LooseCooperationSpec = (
    CooperationSpec | CooperationLevel | CooperationLevelStr | tuple[Literal["exactly", "at-least"], CooperationLevelStr] | bool
)


@overload
def generate(
    kind: Literal["random", "constructive"],
    *,
    height: int = 5,
    width: int = 5,
    n_agents: int = 2,
    n_lasers: int | Literal["auto"] = "auto",
    cooperation: LooseCooperationSpec | None = None,
    n_walls: int | Literal["auto"] = "auto",
    t_max: int | Literal["auto"] = "auto",
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
    n_lasers: int | Literal["auto"] = "auto",
    cooperation: LooseCooperationSpec | None = None,
    n_walls: int | Literal["auto"] = "auto",
    t_max: int | Literal["auto"] = "auto",
    seed: int | None = None,
) -> World | None: ...


@overload
def generate(
    kind: Literal["random", "constructive"],
    *,
    height: int = 5,
    width: int = 5,
    n_agents: int = 2,
    n_lasers: int | Literal["auto"] = "auto",
    cooperation: LooseCooperationSpec | None = None,
    n_walls: int | Literal["auto"] = "auto",
    t_max: int | Literal["auto"] = "auto",
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
    cooperation: LooseCooperationSpec | None = ("exactly", "mutual"),
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
    cooperation: LooseCooperationSpec | None = ("exactly", "mutual"),
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
    max_attempts: int = 10_000,
    height: int = 12,
    width: int = 13,
    n_agents: int = 4,
    n_lasers: int = 3,
    cooperation: LooseCooperationSpec | None = ("exactly", "mutual"),
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
    n_lasers: int | Literal["auto"] = "auto",
    cooperation: LooseCooperationSpec | None = None,
    t_max: int | Literal["auto"] = "auto",
    n_walls: int | Literal["auto"] = "auto",
    seed: int | None = None,
    max_attempts: int | None = None,
    n_jobs: int | Literal["auto"] = "auto",
    n: int = 1,
):
    """
    Build a solvable `World` on demand using a SAT-verified procedural generator.

    Raises:
        - `ValueError` if `kind` is unknown or arguments are invalid;
        - `ValueError` if `cooperative` is passed to `kind="level6_style"`;

    Parameters:
    ----------
    - `kind`: the kind of generator to use. Refer to generator calsses in the `lle.generator` module for more information.
    - `cooperation`: The required level of cooperation. Can be specified as:
        - a boolean: `True` for any cooperative level, `False` for no cooperation;
        - a `CooperationLevel` or a `CooperationLevelStr` (e.g. `CooperationLevel.MUTUAL` or `"mutual"` for exactly mutual);
        - a tuple `(constraint, level)` where `constraint` is either `"exactly"` or `"at-least"` and `level` is a `CooperationLevel` or a `CooperationLevelStr`.
    - `n_walls`: the number of walls to place. If `"auto"`, 10% of the grid is filled with walls.
    - `t_max`: the maximal solution path length. If `"auto"`, defaults to `width * height // 2`.
    - `n`: the number of worlds to generate.
    - `n_jobs`: the number of parallel jobs to run. If `auto`, spawns `n_cpus - 1` jobs.

    Examples:
    --------
    ```python
    import lle
    world = lle.generate("level6_style", n_agents=4, n_lasers=3)
    world = lle.generate("random", n=10, width=5, height=7, n_agents=2, seed=0)
    world = lle.generate("random", cooperation=("at-least", "mutual"), seed=0)
    world = lle.generate("random", cooperation=CooperationLevel.FULLY_COUPLED, seed=0)
    ```
    """
    args = _GenerateArgs(
        kind=kind,
        height=height,
        width=width,
        n_agents=n_agents,
        n_lasers=n_lasers,
        cooperation=cooperation,
        t_max=t_max,
        n_walls=n_walls,
        max_attempts=max_attempts,
        n_jobs=n_jobs,
    ).resolve(n)
    if kind == "random":
        generator = _RandomGenerator(
            height=args.height,
            width=args.width,
            n_agents=args.n_agents,
            n_lasers=args.n_lasers,
            cooperation=args.cooperation,
            n_walls=args.n_walls,
            t_max=args.t_max,
        )
    elif kind == "constructive":
        generator = _ConstructiveGenerator(
            height=args.height,
            width=args.width,
            n_agents=args.n_agents,
            n_lasers=args.n_lasers,
            cooperation=args.cooperation,
            n_walls=args.n_walls,
            t_max=args.t_max,
        )
    elif kind == "level6_style":
        generator = _Level6StyleGenerator(
            height=args.height,
            width=args.width,
            n_agents=args.n_agents,
            n_lasers=args.n_lasers,
            n_walls=args.n_walls,
            t_max=args.t_max,
            cooperation=args.cooperation,
        )
    else:
        raise ValueError(f"Unknown kind: {kind!r}. Expected 'random', 'constructive', or 'level6_style'.")
    if args.n_jobs > 1:
        worlds = generator.generate_n(n, args.n_jobs, seed)
        if n == 1:
            return worlds[0]
        return worlds
    return generator.generate(seed=seed, max_attempts=max_attempts)


def try_generate(*args, max_attempts: int = 10_000, **kwargs):
    """
    Try to generate one or multiple worlds within `max_attempts` attempts. When generating one environemnt,
    returns `None` if `max_attempts` attempts have been tried. If generating `n`>1 environments, returns the
    list of generated environments so far.
    See `lle.generate`."""
    kwargs["max_attempts"] = max_attempts
    return generate(*args, **kwargs)


__all__ = ["generate", "try_generate"]
