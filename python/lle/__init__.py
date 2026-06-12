"""
# Laser Learning Environment

Laser Learning Environment (LLE) is a multi-agent gridworld for cooperative
multi-agent reinforcement learning. Lasers are the central mechanic:
agents die when they enter an active beam unless their colour matches the one
of the laser, in which case they can block the beam and let others pass safely.

![LLE](../../docs/lvl6-annotated.png)

LLE gives you two complementary ways to work with a world:
- `World` for low-level, deterministic control of maps, states, and steps.
- `LLE` for a higher-level MARL environment compatible with `marlenv`.
- `generate`, `solve`, `is_cooperative`, and `characterize` for SAT-based generation and analysis.

## Quick start
Create a simple world, run a step, then restore the previous state:

```python
from lle import Action, World

world = World("S0 . G X")
world.reset()
state = world.get_state()
world.step(Action.EAST)
world.set_state(state)
```

Build an environment for MARL experiments:

```python
import lle

env = lle.level(6).obs_type("layered").build()
observation, state = env.reset()
action = env.sample_action()
step = env.step(action)
```

## Main entry points

### `World`
Use `World` when you want precise control over a custom map, a saved state, or
individual actions. It is the most direct interface to the environment.

### `LLE`
Use `LLE` when you want a ready-to-use MARL environment. The usual workflow is
`lle.level(...)`, `lle.from_str(...)`, or `lle.from_file(...)`, followed by
builder methods such as `obs_type(...)`, `state_type(...)`, and `build()`.

### Generation and analysis

- `lle.generate(width, height, n_agents)` returns a fluent `GeneratorBuilder`. Chain layout,
  laser, and behaviour methods, then call `build()` for one world or `take(n)` for many.
- `lle.solve(world, t_max)` finds the shortest joint plan (or `None`).
- `lle.is_cooperative(world)` checks whether laser blocking is required to solve the world.
- `lle.characterize(world, t_max)` proves which agent dependencies *every* plan of length ≤ `t_max` requires.


```python
import lle
from lle import World

world = lle.generate(width=5, height=5, n_agents=2).build(seed=0)
plan = lle.solve(world, 5)
assert plan is not None
world.reset()
for joint_action in plan:
    world.step(joint_action)
assert lle.is_cooperative(World.level(6))
```

**Builder options** — layout: `random()`, `lanes()`, `clustered()`, `starts(...)` / `exits(...)`.
Obstacles: `lasers(n, placement=..., span=...)`, `walls(n, style=...)`.
Behaviour: `solvable()` (default), `independent()`, `cooperative(...)`, `mutual(...)`.

```python
lle.generate(width=6, height=6, n_agents=2).lasers(2).cooperative().build()
lle.generate(n_agents=4).clustered().mutual(t_max=21).build()
worlds = list(lle.generate(width=5, height=5, n_agents=2).lasers(1).cooperative().take(10))
```

See `lle.generator` for the full method reference.

## Custom maps

You can create custom maps from either a plain string or a TOML file.
The plain-string format encodes one tile per token and uses whitespace to
separate tiles.

```python
from lle import World

world = World("S0 . G X")
```

TOML maps are useful when you want richer placement rules, such as random start
positions.

## Citing our work
The environment has been presented at [EWRL 2023](https://openreview.net/pdf?id=IPfdjr4rIs) and at [BNAIC 2023](https://bnaic2023.tudelft.nl/static/media/BNAICBENELEARN_2023_paper_124.c9f5d29e757e5ee27c44.pdf) where it received the best paper award.

```
@inproceedings{molinghen2023lle,
  title={Laser Learning Environment: A new environment for coordination-critical multi-agent tasks},
  author={Molinghen, Yannick and Avalos, Raphaël and Van Achter, Mark and Nowé, Ann and Lenaerts, Tom},
  year={2023},
  series={BeNeLux Artificial Intelligence Conference},
  booktitle={BNAIC 2023}
}
```
"""

from .lle import __version__, agent, exceptions, tiles, world  # noqa # prevent import reordering


from .agent import Agent
from .env import LLE
from .generator import generate, GeneratorBuilder
from .observations import ObservationType
from .solver import solve
from .types import AgentId, LaserId, Position
from .world import Action, EventType, World, WorldEvent, WorldState
from .characterization import is_cooperative, characterize

__version__: str
from_file = LLE.from_file
from_str = LLE.from_str
level = LLE.level


__all__ = [
    "AgentId",
    "LaserId",
    "Position",
    "world",
    "exceptions",
    "tiles",
    "agent",
    "env",
    "generator",
    "solver",
    "characterization",
    "observations",
    "Agent",
    "World",
    "WorldState",
    "Action",
    "EventType",
    "WorldEvent",
    "ObservationType",
    "LLE",
    "__version__",
    "from_file",
    "from_str",
    "level",
    "solve",
    "is_cooperative",
    "generate",
    "GeneratorBuilder",
    "characterize",
]
