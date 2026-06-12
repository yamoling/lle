"""Procedural world generation.

The single public entry point is `generate`, which returns a fluent
`GeneratorBuilder`. Describe the world you want by chaining methods, then call a
terminal (`build` for one world, `take` for many):

```python
import lle

world = lle.generate(width=10, height=10, n_agents=3).build(seed=0)
world = lle.generate(width=8, height=8, n_agents=2).lanes().cooperative(t_max=30).build(seed=5)
worlds = list(lle.generate(width=8, height=8, n_agents=3).walls(4, style="shapes").take(10))
```

`WorldFilter` and its subclasses (`Solvable`, `Independent`, `Cooperative`,
`Mutual`) describe behavioural constraints and are accepted by
`GeneratorBuilder.require(...)`; the named methods (`cooperative()`, `mutual()`,
…) cover the common cases without constructing a filter by hand.

The concrete `Generator` implementations (`CustomGenerator`, `RandomGenerator`,
`ConstructiveGenerator`, `Level6StyleGenerator`) remain available for advanced or
direct use, but `generate(...)` is the recommended path.
"""

from __future__ import annotations

from .builder import GeneratorBuilder
from .custom import CustomGenerator
from .generator import Generator
from .world_filter import Cooperative, Independent, Mutual, Solvable, WorldFilter

__all__ = [
    "generate",
    "GeneratorBuilder",
    "WorldFilter",
    "Solvable",
    "Independent",
    "Cooperative",
    "Mutual",
    "Generator",
    "CustomGenerator",
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
      `mutual(...)`, or `require(filter)`.
    - Terminals: `build(...)` for a single `World`, `take(n, ...)` for an
      iterator of worlds.

    # Examples
    ```python
    import lle

    world = lle.generate(width=5, height=5, n_agents=2).build(seed=0)
    world = lle.generate(width=6, height=6, n_agents=2).lasers(2).cooperative().build()
    world = lle.generate(n_agents=4).clustered().mutual(t_max=21).build()
    worlds = list(lle.generate(width=5, height=5, n_agents=2).lasers(1).cooperative().take(10))
    ```
    """
    return GeneratorBuilder(width=width, height=height, n_agents=n_agents)
