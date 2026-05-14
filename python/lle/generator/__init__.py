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
    *,
    kind: Literal["random"],
    size: tuple[int, int] = (5, 5),
    agents: int = 2,
    lasers: int | None = None,
    cooperative: bool = False,
    num_walls: int | None = None,
    t_max: int | None = None,
    seed: int | None = None,
    max_attempts: int = 10_000,
) -> World: ...


@overload
def generate(
    *,
    kind: Literal["constructive"],
    size: tuple[int, int] = (5, 5),
    agents: int = 2,
    lasers: int | None = None,
    cooperative: bool = False,
    num_walls: int | None = None,
    t_max: int | None = None,
    seed: int | None = None,
    max_attempts: int = 10_000,
) -> World: ...


@overload
def generate(
    *,
    kind: Literal["level6_style"],
    size: tuple[int, int] = (13, 13),
    agents: int = 4,
    lasers: int = 3,
    t_max: int = 21,
    num_walls: int | None = None,
    seed: int | None = None,
    max_attempts: int = 10_000,
) -> World: ...


def _resolve_lasers(lasers: int | None, agents: int, cooperative: bool) -> int:
    if lasers is not None:
        return lasers
    if cooperative:
        return max(1, agents - 1)
    return 0


def generate(*, kind, **kwargs) -> World:  # type: ignore[no-redef]
    """
    Build a solvable `World` on demand using a SAT-verified procedural generator.

    `kind` selects the layout strategy: `"random"`, `"constructive"`, or
    `"level6_style"`. See the per-kind `@overload` signatures for the
    accepted parameters and defaults.

    ```python
    import lle
    world = lle.generate(kind="random", size=(5, 5), agents=2, seed=0)
    world = lle.generate(kind="level6_style", agents=4, lasers=3, seed=0)
    ```

    Raises `ValueError` if `kind` is unknown or arguments are invalid;
    `TypeError` if `cooperative` is passed to `kind="level6_style"`;
    `RuntimeError` if no valid world is found within `max_attempts`.
    """
    if kind == "random":
        agents = kwargs.get("agents", 2)
        cooperative = kwargs.get("cooperative", False)
        kwargs["lasers"] = _resolve_lasers(kwargs.get("lasers"), agents, cooperative)
        return _RandomGenerator(**kwargs).generate()
    if kind == "constructive":
        agents = kwargs.get("agents", 2)
        cooperative = kwargs.get("cooperative", False)
        kwargs["lasers"] = _resolve_lasers(kwargs.get("lasers"), agents, cooperative)
        return _ConstructiveGenerator(**kwargs).generate()
    if kind == "level6_style":
        if "cooperative" in kwargs:
            raise TypeError(
                "generate(kind='level6_style') does not accept 'cooperative' "
                "(cooperation is intrinsic; omit the argument)."
            )
        kwargs.setdefault("size", (13, 13))
        kwargs.setdefault("agents", 4)
        kwargs.setdefault("lasers", 3)
        kwargs.setdefault("t_max", 21)
        return _Level6StyleGenerator(**kwargs).generate()
    raise ValueError(
        f"Unknown kind: {kind!r}. Expected 'random', 'constructive', or 'level6_style'."
    )


__all__ = ["generate"]
