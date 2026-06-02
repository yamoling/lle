"""
# Laser Learning Environment

Laser Learning Environment (LLE) is a multi-agent gridworld for cooperative
multi-agent reinforcement learning. Lasers are the central mechanic:
agents die when they enter an active beam unless their colour matches the one
of the laser, in which case they can block the beam and let others pass safely.

![pdoc logo](../../docs/lvl6-annotated.png)

LLE gives you two complementary ways to work with a world:
- `World` for low-level, deterministic control of maps, states, and steps.
- `LLE` for a higher-level MARL environment compatible with `marlenv`.
- `generate`, `solve`, `solve_hybrid`, `solve_sat`, `is_cooperative`, and `cooperation_level` for SAT-based generation and analysis.

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
Install the optional generator extra to use SAT-based generation and solving:

```bash
pip install laser-learning-environment[generator]
```

- `lle.generate(...)` builds a solvable world on demand.
- `lle.solve(world, t_max)` searches for the shortest joint plan that reaches all exits within the time bound.
- `lle.solve_hybrid(world, t_max)` searches for the shortest joint plan using incremental SAT clause reuse.
- `lle.solve_sat(world, t_max)` searches for a joint plan of exactly `t_max` steps.
- `lle.is_cooperative(world, t_max)` checks whether the world requires laser blocking under standard semantics.
- `lle.cooperation_level(world, t_max)` returns the more precise cooperation classification.

`lle.generate(...)` and the solver helpers live in `lle.generator` and
`lle.solver`, but `import lle` re-exports them for convenience.

These helpers raise `ImportError` when the optional `generator` extra is not
installed. `generate(...)` raises `ValueError` when you ask for an impossible
or unsupported configuration.

Example:

```python
import lle
from lle import World

world = lle.generate(kind="random", height=5, width=5, n_agents=2, seed=0)
plan = lle.solve(world, t_max=5)
assert plan is not None
world.reset()
for joint_action in plan:
    world.step(joint_action)
assert lle.is_cooperative(World.level(6))
```

## Procedural generation

`lle.generate(kind=..., **kwargs)` builds a world using one of three procedural generators. See `lle.generator` for the full argument matrix:

- `kind="random"` — random layout. It validates geometry and SAT-verifies solvability.
- `kind="constructive"` — lane-based layout with an explicit constructive solution.
- `kind="level6_style"` — Level-6-inspired clustered starts and exits. This
  kind defaults to an exactly mutual cooperative configuration.

The `cooperation` argument lets you constrain the requested cooperation
behaviour. You can pass `True`, `False`, a `CooperationLevel`, a string such
as `"mutual"`, or a tuple such as `("at-least", "mutual")`. See
`CooperationLevel` for the exact ordering. Use `cooperation=True` when you
want any cooperative world rather than a specific profile.

Examples:

```python
import lle
from lle import CooperationLevel

lle.generate(kind="random", height=5, width=5, n_agents=2)
lle.generate(kind="random", height=6, width=6, n_agents=2, n_lasers=2, cooperation=True)
lle.generate(kind="level6_style", n_agents=4, n_lasers=3, t_max=21)
lle.generate(kind="constructive", n_lasers=2, cooperation=("at-least", CooperationLevel.ASYMMETRIC))
lle.generate(kind="constructive", n_lasers=3, cooperation=("exactly", "asymmetric"))
```

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

## More details

The root package re-exports the most common classes and helpers so that
`import lle` is enough for most use cases. See the module pages for
`World`, `LLE`, `generator`, `solver`, `tiles`, `world`, and `observations`
for the full API.

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
from .env import LLE, make_pool
from .generator import generate
from .observations import ObservationType
from .solver import (
    CooperationLevel,
    CooperationLevelStr,
    cooperation_level,
    cooperation_level_trajectory,
    is_cooperative,
    solve,
    solve_hybrid,
    solve_sat,
)
from .types import AgentId, LaserId, Position
from .world import Action, EventType, World, WorldEvent, WorldState

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
    "solve_hybrid",
    "solve_sat",
    "is_cooperative",
    "cooperation_level",
    "cooperation_level_trajectory",
    "CooperationLevel",
    "generate",
    "make_pool",
    "CooperationLevelStr",
]
