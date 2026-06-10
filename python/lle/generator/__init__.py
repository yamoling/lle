from __future__ import annotations

from typing import Any, Generator, Literal, Unpack, overload

from ..world import World
from ._args import GenerateArgs, _ConfigFilter, _ConfigKW, _ConfigMultipleFilter, _ConfigMultipleKW
from .constructive import ConstructiveGenerator
from .level6_style import Level6StyleGenerator
from .random import RandomGenerator
from .world_filter import WorldFilter

__all__ = ["generate", "WorldFilter"]


@overload
def generate(
    *,
    n_agents: int = 3,
    height: int = 10,
    width: int = 10,
    n_lasers: int = 3,
    cooperative: bool | None = None,
    mutual: bool | None = None,
    t_min: int = 0,
    t_max: int | Literal["auto"] = "auto",
    n_walls: int | Literal["auto"] = "auto",
    seed: int | None = None,
    n_jobs: int | Literal["auto"] = 1,
    kind: Literal["auto", "level6_style", "random", "constructive"] = "auto",
    max_attempts: None = None,
) -> World: ...


@overload
def generate(**args: Unpack[_ConfigFilter]) -> World: ...
@overload
def generate(*, max_attempts: int, **args: Unpack[_ConfigFilter]) -> World | None: ...
@overload
def generate(*, max_attempts: int, **args: Unpack[_ConfigKW]) -> World | None: ...
@overload
def generate(**args: Unpack[_ConfigMultipleKW]) -> Generator[World, Any, None]: ...
@overload
def generate(**args: Unpack[_ConfigMultipleFilter]) -> Generator[World, Any, None]: ...


def generate(
    *,
    n: int = 1,
    n_agents: int = 3,
    height: int = 10,
    width: int = 10,
    n_lasers: int | Literal["auto"] = 3,
    cooperative: bool | None = None,
    mutual: bool | None = None,
    filter: WorldFilter | None = None,
    t_min: int = 0,
    t_max: int | Literal["auto"] = 20,
    n_walls: int | Literal["auto"] = "auto",
    seed: int | None = None,
    n_jobs: int | Literal["auto"] = 1,
    kind: Literal["auto", "level6_style", "random", "constructive"] = "level6_style",
    max_attempts: int | None = None,
    quiet: bool = False,
):
    """
    Build solvable `World` instances using a SAT-verified procedural generator.

    # Parameters
    - `kind`: generator strategy — ``"random"``, ``"constructive"``, or ``"level6_style"``.
    - `n`: number of worlds to produce.
    - `filter`: a `WorldFilter` expressing world constraints (cooperative, mutual, ...).
      Cannot be combined with `cooperation` or `mutual`.
    - `cooperative`: shortcut for ``filter=WorldFilter(cooperative=...)``.
    - `mutual`: shortcut for ``filter=WorldFilter(mutual=...)``.
    - `n_walls`: number of walls; ``"auto"`` fills 10 % of the grid with walls.
    - `t_min`: minimum solution path length in the solver.
    - `t_max`: upper bound on the solution length; ``"auto"`` uses ``width * height // 2``.
    - `n_jobs`: parallel workers; ``"auto"`` uses all CPUs minus one when ``n > 1``.
    - `max_attempts`: budget of generation attempts before giving up.
    - `quiet`: suppress the progress bar when ``n > 1``.

    # Returns
    - `n=1`, no `max_attempts`: a single `World`.
    - `n=1`, `max_attempts` set: a `World` or `None`.
    - `n>1`: a generator of up to `n` worlds.

    # Examples
    ```python
    import lle
    from lle.generator import WorldFilter

    world = lle.generate(height=5, width=5, n_agents=2, seed=0)
    world = lle.generate(cooperative=True, n_agents=2, height=5, width=5, n_lasers=1)
    world = lle.generate(mutual=True, kind="constructive", n_agents=4)
    worlds = list(lle.generate(n=10, filter=WorldFilter(cooperative=True), kind="random",
                               height=5, width=5, n_agents=2, n_lasers=1))
    ```
    """
    if filter is not None and (cooperative is not None or mutual is not None):
        raise ValueError(
            "Cannot combine a WorldFilter with the 'cooperation' or 'mutual' keyword arguments. "
            "Either pass a WorldFilter or use the keyword shortcuts, not both."
        )
    args = GenerateArgs(
        kind=kind,
        height=height,
        width=width,
        n_agents=n_agents,
        n_lasers=n_lasers,
        cooperation=cooperative,
        mutual=mutual,
        filter=filter,
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
            world_filter=args.filter,
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
            world_filter=args.filter,
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
            world_filter=args.filter,
        )
    else:
        raise ValueError(f"Unknown kind: {kind!r}. Expected 'random', 'constructive', or 'level6_style'.")

    if args.n_jobs > 1:
        worlds = generator.generate_n(n, args.n_jobs, seed, args.max_attempts, quiet=quiet)
        if n == 1:
            return next(worlds)
        return worlds
    return generator.generate(seed=seed, max_attempts=max_attempts)
