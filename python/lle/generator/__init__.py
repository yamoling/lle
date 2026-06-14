"""Procedural world generation.

The single public entry point is `generate`, which returns a fluent
`GeneratorBuilder`. Describe the world you want by chaining methods, then call a
terminal (`build` for one world, `take` for many):

```python
world = lle.generate(width=5, height=5, n_agents=3).build(seed=0)
world = lle.generate(width=4, height=4, n_agents=2).lanes().cooperative(t_max=12).build(seed=5)
worlds = list(lle.generate(width=5, height=5, n_agents=2).walls(2, style="shapes").take(3))
```

`WorldFilter` and its subclasses (`Solvable`, `Independent`, `Cooperative`,
`Chained`, `Mutual`) describe behavioural constraints and are accepted by
`GeneratorBuilder.require(...)`; the named methods (`cooperative()`, `chained()`,
`mutual()`, …) cover the common cases without constructing a filter by hand.

`CustomGenerator` remains available for advanced or direct use, but
`generate(...)` is the recommended path.
"""

from __future__ import annotations

from .builder import GeneratorBuilder
from .generator import WorldGenerator
from .world_filter import Chained, Cooperative, Independent, Interdependent, Mutual, Solvable, WorldFilter

__all__ = [
    "generate",
    "GeneratorBuilder",
    "WorldFilter",
    "Solvable",
    "Independent",
    "Cooperative",
    "Chained",
    "Mutual",
    "Interdependent",
    "WorldGenerator",
]


def generate(width: int = 10, height: int = 10, n_agents: int = 3) -> GeneratorBuilder:
    """Start a world-generation request.

    Returns a fluent `GeneratorBuilder` configured for a `width` × `height` grid
    with `n_agents` agents. Chain configuration methods and finish with a
    terminal:

    - Layout: `random()`, `lanes()`, `clustered()`, or fine-grained
      `starts(...)` / `exits(...)`.
    - Lasers and walls: `lasers(...)`, `walls(...)`.
    - Behaviour: `solvable()` (default), `independent()`, `cooperative(...)`,
      `chained(...)`, `mutual(...)`, or `require(filter)`.
    - Terminals: `build(...)` for a single `World`, `take(n, ...)` for an
      iterator of worlds.

    # Examples
    ```python
    world = lle.generate(width=5, height=5, n_agents=2).build(seed=0)
    world1 = lle.generate(width=5, height=5, n_agents=2).lasers(1).cooperative(t_max=10).build()
    world2 = lle.generate(width=5, height=5, n_agents=2).clustered().lasers(2).mutual(t_max=10).build()
    worlds = lle.generate(width=4, height=4, n_agents=2).starts("edge").exits("opposite").lasers(1).cooperative(t_max=10).take(2)
    ```
    """
    return GeneratorBuilder(width=width, height=height, n_agents=n_agents)
