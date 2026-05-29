"""Procedural world generation helpers.

Use `generate` to build a solvable `World` on demand. The module exposes
three generator families:

- `random`: samples arbitrary layouts and validates them with the SAT solver.
- `constructive`: builds a layout with an explicit constructive solution.
- `level6_style`: builds a Level-6-inspired cooperative layout. It defaults to an exactly mutual cooperative configuration.

The `cooperation` argument accepts either a boolean, a `CooperationLevel`, a
string such as `"mutual"`, or a tuple like `("at-least", "cooperative")`.
See `CooperationLevel` for the ordering used by the solver. Use
`cooperation=True` when you want any cooperative world rather than a specific
profile.
"""

from __future__ import annotations

from typing import Literal, overload

from ..solver import _require_pysat
from ..solver.cooperation_level import CooperationLevel, CooperationLevelStr
from ..world import World
from ._args import _GenerateArgs
from ._base import CooperationSpec
from .constructive import ConstructiveGenerator
from .level6_style import Level6StyleGenerator
from .random import RandomGenerator

LooseCooperationSpec = (
    CooperationSpec | CooperationLevel | CooperationLevelStr | tuple[Literal["exactly", "at-least"], CooperationLevelStr] | bool
)


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
    n_jobs: int | Literal["auto"] = 1,
) -> World: ...


@overload
def generate(
    kind: Literal["level6_style"],
    *,
    max_attempts: int,
    n: int = 1,
    height: int = 12,
    width: int = 13,
    n_agents: int = 4,
    n_lasers: int = 3,
    cooperation: LooseCooperationSpec | None = ("exactly", "mutual"),
    t_max: int = 21,
    n_walls: int | Literal["auto"] = "auto",
    seed: int | None = None,
    n_jobs: int | Literal["auto"] = "auto",
) -> World | None: ...


@overload
def generate(
    kind: Literal["level6_style"] = "level6_style",
    *,
    n: int,
    max_attempts: int | None = None,
    height: int = 12,
    width: int = 13,
    n_agents: int = 4,
    n_lasers: int = 3,
    cooperation: LooseCooperationSpec | None = ("exactly", "mutual"),
    t_max: int = 21,
    n_walls: int | Literal["auto"] = "auto",
    seed: int | None = None,
    n_jobs: int | Literal["auto"] = "auto",
    quiet: bool = False,
) -> list[World]: ...


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
    n_jobs: int | Literal["auto"] = 1,
) -> World: ...


@overload
def generate(
    kind: Literal["random", "constructive"],
    *,
    max_attempts: int,
    height: int = 5,
    width: int = 5,
    n_agents: int = 2,
    n_lasers: int | Literal["auto"] = "auto",
    cooperation: LooseCooperationSpec | None = None,
    n_walls: int | Literal["auto"] = "auto",
    t_max: int | Literal["auto"] = "auto",
    seed: int | None = None,
    n_jobs: int | Literal["auto"] = 1,
) -> World | None: ...


@overload
def generate(
    kind: Literal["random", "constructive"],
    *,
    n: int,
    max_attempts: int | None = None,
    height: int = 5,
    width: int = 5,
    n_agents: int = 2,
    n_lasers: int | Literal["auto"] = "auto",
    cooperation: LooseCooperationSpec | None = None,
    n_walls: int | Literal["auto"] = "auto",
    t_max: int | Literal["auto"] = "auto",
    seed: int | None = None,
    n_jobs: int | Literal["auto"] = "auto",
    quiet: bool = False,
) -> list[World]: ...


def generate(
    kind: Literal["random", "constructive", "level6_style"] = "level6_style",
    *,
    n: int = 1,
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
    quiet: bool = False,
):
    """
    Build a solvable `World` on demand using a SAT-verified procedural generator.

    Parameters:
    ----------
    - `kind`: the kind of generator to use. Refer to generator calsses in the `lle.generator` module for more information.
    - `cooperation`: The required level of cooperation. Can be specified as:
        - a boolean: `True` for any cooperative level, `False` for no cooperation;
        - a `CooperationLevel` or a `CooperationLevelStr` (e.g. `CooperationLevel.MUTUAL` or `"mutual"` for exactly mutual);
        - a tuple `(constraint, level)` where `constraint` is either `"exactly"` or `"at-least"` and `level` is a `CooperationLevel` or a `CooperationLevelStr`.
    - `n_walls`: the number of walls to place. If `"auto"`, 10% of the grid is filled with walls.
    - `t_max`: the maximal solution path length. If `"auto"`, defaults to `width * height // 2`.
    - `max_attempts`: the maximum number of attempts to generate valid worlds before stopping. If `None`, there is no limit.
    - `n`: the number of worlds to generate
    - `n_jobs`: the number jobs to run in parallel. When `auto`, spawns `n_cpus - 1` jobs if `n` > 1 or spawns 1 job is `n` = 1.
    - `quiet`: whether to remove the progress bar when `n` > 1.

    Returns:
    --------
        - When `max_attempts` is not provided (default):
            - A single `World` if `n=1`;
            - A list of `n` `World` if `n` > 1.
        - When `max_attempts` is set:
            - A `World | None` for `n` = 1;
            - A list of at most `n` worlds `World`s for `n` > 1.

    Raises:
    -------
        - `ValueError` if arguments are invalid;
        - `ValueError` if there is no default argument value for the given combination.

    Examples:
    --------
    ```python
    import lle
    world = lle.generate("level6_style", n_agents=4, n_lasers=3)
    world = lle.generate("random", n=10, width=5, height=7, n_agents=2, seed=0)
    world = lle.generate("random", cooperation=("at-least", "mutual"))
    world = lle.generate("random", cooperation=CooperationLevel.FULLY_COUPLED, seed=0)
    world_or_none = lle.genrate("random", max_attempts=10, cooperation="mutual")
    ```
    """
    _require_pysat()
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
        generator = RandomGenerator(
            height=args.height,
            width=args.width,
            n_agents=args.n_agents,
            n_lasers=args.n_lasers,
            cooperation=args.cooperation,
            n_walls=args.n_walls,
            t_max=args.t_max,
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
        )
    elif kind == "level6_style":
        generator = Level6StyleGenerator(
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
        worlds = generator.generate_n(n, args.n_jobs, seed, args.max_attempts, quiet=quiet)
        if n == 1:
            return worlds[0]
        return worlds
    return generator.generate(seed=seed, max_attempts=max_attempts)
