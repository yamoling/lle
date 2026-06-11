from __future__ import annotations

from collections.abc import Iterator
from typing import Literal, overload

from ..world import World
from ._args import GenerateArgs
from .constructive import ConstructiveGenerator
from .generator import Generator
from .level6_style import Level6StyleGenerator
from .random import RandomGenerator
from .world_filter import Cooperative, GeneratorKind, Independent, Mutual, Solvable, WorldFilter

__all__ = ["generate", "WorldFilter", "Solvable", "Independent", "Cooperative", "Mutual"]

_GENERATORS: dict[GeneratorKind, type[Generator]] = {
    "random": RandomGenerator,
    "constructive": ConstructiveGenerator,
    "level6_style": Level6StyleGenerator,
}

_Kind = Literal["auto", "random", "constructive", "level6_style"]


# A single world is returned when ``n == 1`` and the attempt budget is unbounded.
@overload
def generate(
    kind: _Kind = "auto",
    *,
    n: Literal[1] = 1,
    max_attempts: None = None,
    n_agents: int = 3,
    height: int = 10,
    width: int = 10,
    n_lasers: int | Literal["auto"] = "auto",
    cooperative: bool | None = None,
    mutual: bool | None = None,
    filter: WorldFilter | None = None,
    t_min: int | None = None,
    t_max: int | Literal["auto"] = "auto",
    n_walls: int | Literal["auto"] = "auto",
    seed: int | None = None,
    n_jobs: int | Literal["auto"] = 1,
    quiet: bool = False,
) -> World: ...


# A bounded budget may fail, so a single world becomes optional.
@overload
def generate(
    kind: _Kind = "auto",
    *,
    n: Literal[1] = 1,
    max_attempts: int,
    n_agents: int = 3,
    height: int = 10,
    width: int = 10,
    n_lasers: int | Literal["auto"] = "auto",
    cooperative: bool | None = None,
    mutual: bool | None = None,
    filter: WorldFilter | None = None,
    t_min: int | None = None,
    t_max: int | Literal["auto"] = "auto",
    n_walls: int | Literal["auto"] = "auto",
    seed: int | None = None,
    n_jobs: int | Literal["auto"] = 1,
    quiet: bool = False,
) -> World | None: ...


# Asking for several worlds yields a (possibly shorter than ``n``) stream.
@overload
def generate(
    kind: _Kind = "auto",
    *,
    n: int,
    max_attempts: int | None = None,
    n_agents: int = 3,
    height: int = 10,
    width: int = 10,
    n_lasers: int | Literal["auto"] = "auto",
    cooperative: bool | None = None,
    mutual: bool | None = None,
    filter: WorldFilter | None = None,
    t_min: int = 0,
    t_max: int | Literal["auto"] = "auto",
    n_walls: int | Literal["auto"] = "auto",
    seed: int | None = None,
    n_jobs: int | Literal["auto"] = 1,
    quiet: bool = False,
) -> Iterator[World]: ...


def generate(
    kind: _Kind = "auto",
    *,
    n: int = 1,
    max_attempts: int | None = None,
    n_agents: int = 3,
    height: int = 10,
    width: int = 10,
    n_lasers: int | Literal["auto"] = "auto",
    cooperative: bool | None = None,
    mutual: bool | None = None,
    filter: WorldFilter | None = None,
    t_min: int | None = None,
    t_max: int | Literal["auto"] = "auto",
    n_walls: int | Literal["auto"] = "auto",
    seed: int | None = None,
    n_jobs: int | Literal["auto"] = 1,
    quiet: bool = False,
) -> World | Iterator[World] | None:
    """Build solvable `World` instances using a SAT-verified procedural generator.

    # Parameters
    - `kind`: generator strategy — ``"random"``, ``"constructive"``, ``"level6_style"``,
      or ``"auto"`` to let the filter pick the best fit for the provided arguments.
    - `n`: number of worlds to produce.
    - `filter`: a `WorldFilter` describing the desired world (e.g. `Cooperative()`,
      `Mutual()`, `Independent()`). Mutually exclusive with `cooperative` / `mutual`.
    - `cooperative`: shortcut — ``True`` → `Cooperative()`, ``False`` → `Independent()`.
    - `mutual`: shortcut — ``True`` → `Mutual()`.
    - `n_lasers`: number of laser sources; ``"auto"`` picks a sensible count for the filter.
    - `n_walls`: number of walls; ``"auto"`` fills ~10% of the grid.
    - `t_min`: if provided, reject worlds solvable in fewer than ``t_min`` steps.
    - `t_max`: upper bound on the solution length; ``"auto"`` uses ``width * height // 2``.
    - `n_jobs`: parallel workers; ``"auto"`` uses all CPUs minus one when ``n > 1``.
    - `max_attempts`: budget of generation attempts before giving up.
    - `quiet`: suppress the progress bar when ``n > 1``.

    # Returns
    - ``n == 1`` with no `max_attempts`: a single `World`.
    - ``n == 1`` with `max_attempts`: a `World` or `None`.
    - ``n > 1``: an iterator of up to `n` worlds.

    # Examples
    ```python
    import lle
    from lle.generator import Cooperative, Mutual, WorldFilter

    world = lle.generate(height=5, width=5, n_agents=2, seed=0)
    world = lle.generate(cooperative=True, n_agents=2, height=5, width=5, n_lasers=1)
    world = lle.generate("constructive", mutual=True, n_agents=4)
    worlds = list(lle.generate(n=10, filter=Cooperative(), kind="random",
                               height=5, width=5, n_agents=2, n_lasers=1))
    ```
    """
    args = GenerateArgs(
        kind=kind,
        height=height,
        width=width,
        n_agents=n_agents,
        n_lasers=n_lasers,
        t_min=t_min,
        t_max=t_max,
        n_walls=n_walls,
        max_attempts=max_attempts,
        n_jobs=n_jobs,
        filter=filter,
        cooperative=cooperative,
        mutual=mutual,
    ).resolve(n)

    generator = _GENERATORS[args.kind](
        height=args.height,
        width=args.width,
        n_agents=args.n_agents,
        n_lasers=args.n_lasers,
        world_filter=args.filter,
        n_walls=args.n_walls,
    )

    if n == 1 and args.n_jobs == 1:
        return generator.generate(max_attempts=args.max_attempts, seed=seed)

    worlds = generator.generate_n(n=n, n_jobs=args.n_jobs, seed=seed, max_attempts=args.max_attempts, quiet=quiet)
    if n == 1:
        return next(worlds, None)
    return worlds
