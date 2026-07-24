"""
# Laser Learning Environment

Laser Learning Environment (LLE) is a multi-agent gridworld for cooperative
multi-agent reinforcement learning. Lasers are the central mechanic:
agents die when they enter an active beam unless their colour matches the one
of the laser, in which case they can block the beam and let others pass safely.

![LLE](../../docs/lvl6-annotated.png)

LLE provides two complementary ways to work with a world:
- `World` for low-level control of maps, states, and steps.
- `LLE` for a higher-level MARL environment compatible with the [`marlenv` library](https://github.com/yamoling/multi-agent-rlenv).
- `generate`, `solve`, `is_cooperative`, `is_convergent`, and `characterize` for SAT-based world generation and analysis.

## Low-level `World`
Use `World` when you want precise control over a custom map, a saved state, or
individual actions. It is the most direct interface to the environment.


Create a simple world, run a step, then restore the previous state:
```python
from lle import Action, World

level3 = World.level(3)
world = World("S0 . G X")
world.reset()
state = world.get_state()
world.step(Action.EAST)
print(world.agents_positions)  # [(0, 1)]
world.set_state(state)
print(world.exit_pos)          # [(0, 3)]
```

## High-level `LLE`
Use `LLE` when you want a ready-to-use MARL environment with multiple types of observations (partial, full, 1d, 3d, ...).
The usual workflow is `lle.level(...)`, `lle.from_str(...)`, or `lle.from_file(...)`, followed by
builder methods such as `obs_type(...)`, `state_type(...)`, and `build()`.


Build an environment for MARL experiments:
```python
env = lle.level(6).obs_type("layered").build()
observation, state = env.reset()
action = env.sample_action()
step = env.step(action)
print(step.reward)
```

## Solving a world
`lle.solve(world, t_max)` finds a shortest joint plan (sequence of joint actions) within `t_max` steps,
or `None` if such plan does not exist. This solving is performed via a SAT solver.
More details about the solver can be found in the `lle.solver` module.

```python
world = lle.World.level(5)
plan = lle.solve(world, 20)
assert plan is not None
world.reset()
for joint_action in plan:
    world.step(joint_action)
```

## Characterizing a world
The cooperation requirements of a world can be characterized by calling `lle.characterize(world, t_max)`,
which returns a `WorldCharacterizer` that can be queried to determine the intrinsic cooperative properties
of the world (i.e. every solution to the world within `t_max` steps has these properties).

Helper functions such as `lle.is_cooperative`, `lle.is_interdependent(k)` or `lle.is_convergent(world, k=2)` can also be used. Note that if multiple properties
should be checked, it is more efficient to use `lle.characterize` and query the `WorldCharacterizer` because it
avoids redundant recomputation of the same properties.

```python
specs = lle.characterize(World.level(6), t_max=21)
assert not specs.is_independent()
assert specs.is_cooperative()
assert specs.is_interdependent(2)
assert not lle.is_cooperative(World.level(1))
assert lle.is_cooperative(World.level(3))
assert not lle.is_interdependent(World.level(3), 2, t_max=21)
assert not lle.is_convergent(World.level(3), t_max=21)
```


## World procedural generation
The `lle.generator` module provides procedural world generation functionalities. The simplest
way to generate a world is by calling `lle.generate(width, height, n_agents).build()`.
`lle.generate(...)` returns a `GeneratorBuilder` that can be chained with parameters to
customize the layout generation. Call `build()` or `take(n)` to generate one or multiple worlds respectively.

```python
gen = lle.generate(width=5, height=5, n_agents=2)
world = gen.build()         # One single world
worlds = list(gen.take(3))  # Three worlds
builder = lle.generate(width=4, height=4, n_agents=2).lasers(1).cooperative().cap(10)
builder = lle.generate(width=4, height=4, n_agents=2).lasers(2).interdependent(2).cap(10)
two_worlds = list(lle.generate(width=4, height=4, n_agents=2).lasers(1).cooperative().cap(10).take(2))
```

**Builder options** — layout: `random()`, `lanes()`, `clustered()`, `starts(...)` / `exits(...)`.
Obstacles: `lasers(n, placement=..., span=...)`, `walls(n, style=...)`.
Behaviour: `solvable()` (default), `independent()`, `cooperative()`, `chained(n)`, `interdependent(n)`, or `require(...)`.

See `lle.generator` for the full method reference.

## Custom maps

You can create custom maps from either a plain string or a TOML file.
The plain-string format encodes one tile per token and uses whitespace to
separate tiles.

| Character | Tile | Walkable | Comment |
------------|------|----------|---------|
| `.` | Floor | Yes | The most basic tile. |
| `@` | Wall  | No | A wall that blocks lasers. |
| `X` | Exit  | Yes | An exit tile. The agent can no longer move after reaching it. |
| `G` | Gem   | Yes | A gem to collect. |
| `S<n>` | Start | Yes | Start position of agent `n`. |
| `L<n><d>` | Laser source | No | Source of a laser of colour `n` (a number) beaming toward the direction `d` (N, S, E, W). |
| `V` | Void | Yes | A void tile. The agent dies if it walks on it |

For instance, the following map string yields the image shown below.

```text
S0 . G . X
S1 @ . . .
L0E . . V V
@  @ . V V
G  . . . X
```
![pdoc logo](../../docs/example_custom.png)

TOML maps are useful when you want richer placement rules, such as random start
positions. Positions can be specified as a list of positions `{i, j}` or as rectangles `{i_min, i_max, j_min, j_max}`,
and the `world_string` field can be used to define the map as discussed above. The full list of authorized
keys is defined by the JSON schema as shown in the below example.

```toml
#:schema https://raw.githubusercontent.com/yamoling/lle/refs/heads/master/resources/lle_toml_schema.json
width = 10 # Optional, deduced from `world_string`
height = 5 # Optional, deduced from `world_string`
exits = [{ j_min = 9 }] # Exits on all cells with j>=9
gems = [{ i = 0, j = 2 }] # One single gem at position (0, 2)
starts = [{ row = 2}] # All tiles on row=2 are start positions for all agents
world_string = '''
X . . . S1 . . . . .
. . . . .  . . . . .
. . . . .  . . . . .
. . . . .  . . . . .
. . . . S2 . . . . .
'''

[[agents]]
# Define a rectangle of possible start positions with both ends included.
# The default minimal value is 0.
# The default maximal value is the width (for j) or height (for i) of the map.
starts = [{ i_min = 0, i_max = 2 }] # Rectangle from (0, 0) to (2, 4) included

[[agents]]
# Deduced from the string map that agent 1 has a start position at (0, 5).

[[agents]]
# Can either start on the 2nd row or on the 7th column.
starts = [{ row = 2 }, { col = 7 }]

[[agents]]
# Start positions can be a mix of rectangles, rows, columns and positions.
starts = [
    { i = 4, j = 9 },
    { i_min = 1, i_max = 3, j_min = 0, j_max = 3 },
    { j_min = 4 },
]
```

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

from .lle import *  # noqa -> prevent import reordering


from .agent import Agent
from .env import LLE
from .generator import generate, GeneratorBuilder
from .observations import ObservationType
from .solver import solve
from .types import AgentId, LaserId, Position
from .world import Action, EventType, World, WorldEvent, WorldState
from .characterization import is_cooperative, characterize, is_asymmetric, is_chained, is_convergent
from . import tiles, exceptions, world, agent, env, generator, characterization, solver, observations


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
    "is_asymmetric",
    "is_chained",
    "is_convergent",
    "generate",
    "GeneratorBuilder",
    "characterize",
]
