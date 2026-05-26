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
    n: int,
    height: int = 5,
    width: int = 5,
    n_agents: int = 2,
    n_lasers: int | Literal["auto"] = "auto",
    cooperation: LooseCooperationSpec | None = None,
    n_walls: int | Literal["auto"] = "auto",
    t_max: int | Literal["auto"] = "auto",
    seed: int | None = None,
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
    n_jobs: int | Literal["auto"] = 1,
) -> World: ...


@overload
def generate(
    kind: Literal["level6_style"] = "level6_style",
    *,
    n: int,
    height: int = 12,
    width: int = 13,
    n_agents: int = 4,
    n_lasers: int = 3,
    cooperation: LooseCooperationSpec | None = ("exactly", "mutual"),
    t_max: int = 21,
    n_walls: int | Literal["auto"] = "auto",
    seed: int | None = None,
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
):
    """Build one or more worlds using a SAT-verified procedural generator.

    Use `kind="random"` for unconstrained sampling, `kind="constructive"` for
    lane-based layouts, and `kind="level6_style"` for a cooperative layout
    inspired by the canonical Level 6.

    `cooperation` lets you require a specific cooperation profile. For
    `kind="level6_style"`, the generator always produces a cooperative world,
    so passing `cooperation=None` keeps the default profile.

    Raises `ValueError` when the requested configuration is impossible or when
    you pass an unsupported `kind`.
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
        worlds = generator.generate_n(n, args.n_jobs, seed)
        if n == 1:
            return worlds[0]
        return worlds
    return generator.generate(seed=seed, max_attempts=max_attempts)


def try_generate(*args, max_attempts: int = 10_000, **kwargs):
    """Try to generate a world without raising when sampling fails.

    When `n == 1`, return `None` if the generator exhausts `max_attempts`.
    When `n > 1`, return the worlds collected so far.
    """
    kwargs["max_attempts"] = max_attempts
    return generate(*args, **kwargs)
