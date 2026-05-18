"""
# Laser Learning Environment
The Laser Learning Environment (LLE) is a multi-agent gridworld aimed for
research in cooperative multi-agent reinforcement learning. The main dynamic of LLE revolves around lasers since walking in a laser is deadly for all agents except those of the same colour as the laser. When an agent of the same colour as the laser walks in a laser, the laser is blocked and other agents can safely walk through it.

LLE has diﬀerent types of tiles: floor, start, wall, laser, laser source, void, gem, and exit tiles, as illustrated in the below image.

![pdoc logo](../../docs/lvl6-annotated.png)


## General-purpose World
The `World` class is at the heart of LLE and is meant for fine-grained control of the environment, as opposed to the `LLE` class, meant for generic high-level control in multi-agent reinforcement learning.

```python
from lle import World, Action

# A map with one agent, one gem and one exit.
world = World("S0 . G X")
world.reset()
state = world.get_state()
world.step([Action.EAST])
world.set_state(state)
```

## Cooperative MARL LLE
The `LLE` class is meant for Multi-Agent Reinforcement Leanring (MARL) and to be used with the `multi-agent-rlenv` library. As such, it encapsulates the `World` class and provides a high-level API for multi-agent reinforcement learning. `LLE` can be used with either in single-objective or multi-objective mode as shown below.

```python
import lle

env = lle.level(6).obs_type("layered").build()
obs = env.reset()
action = env.sample_action()
env.step(action)
```

## Procedural generation, solving, and cooperation analysis
LLE ships an optional SAT-based generator and solver. They require the `[generator]` extra:
```bash
pip install laser-learning-environment[generator]
```

### Generating a world
`lle.generate(kind=..., **kwargs)` builds a solvable `World` on demand. Three kinds are available:

- `kind="random"` — random layout. Validates geometry (no laser pointing out of bounds, no exit on a beam tile) and SAT-verifies solvability.
- `kind="constructive"` — lane-based layout that guarantees a constructive solution.
- `kind="level6_style"` — LLE-Level-6-inspired clustered starts/exits. Always produces a cooperation-requiring world.

Pass `cooperative=True` on `random` or `constructive` to require a same-colour laser blocking situation; `lasers` is then coerced into `[1, agents]`. `level6_style` is always cooperative — passing `cooperative=...` raises `TypeError`.

```python
import lle

# Random 5x5 with 2 agents, no lasers.
world = lle.generate(kind="random", size=(5, 5), agents=2, seed=0)

# Cooperative 6x6 world (standard SAT solvable AND strict-laser UNSAT).
coop = lle.generate(kind="random", size=(6, 6), agents=2, lasers=2,
                    cooperative=True, seed=0)

# Level-6-style 13x13 cooperative world.
big = lle.generate(kind="level6_style", agents=4, lasers=3, t_max=21, seed=0)
```

### Solving a world
`lle.solve(world, t_max=None)` returns a joint plan that brings every agent to an exit within `t_max` steps, or `None` if unsolvable. The plan is a list of length `t_max`; each entry is a tuple of `Action` (one per agent). Default `t_max = (width * height) // 2`.

```python
import lle
from lle import World

world = World("S0 . . X")
plan = lle.solve(world, t_max=5)   # list[tuple[Action, ...]] of length 5
assert plan is not None
world.reset()
for joint_action in plan:
    world.step(list(joint_action))

# Unsolvable: agent walled off from the exit.
assert lle.solve(World("S0 @ X"), t_max=10) is None
```

### Cooperation check
`lle.is_cooperative(world, t_max=None)` returns `True` iff the level is solvable under standard laser semantics **and** unsolvable under strict-laser semantics (where an agent of colour `c` may not block a laser of colour `c`). This is the same check used internally by `cooperative=True` generation.

```python
import lle
from lle import World

assert lle.is_cooperative(World.level(6)) is True          # canonical coop level
assert lle.is_cooperative(World("S0 . X")) is False        # trivial single-agent
```

### Precise cooperation classification
`lle.cooperation_level(world, t_max=None)` refines the binary `is_cooperative` check into a `CooperationLevel` enum (`UNSOLVABLE`, `INDEPENDENT`, `COOPERATIVE`, `ASYMMETRIC`, `MUTUAL`, `CHAIN`, `DISTRIBUTED`, `FULLY_COUPLED`). See `CooperationLevel`'s docstring for the structural meaning of each member.

```python
import lle
from lle import CooperationLevel, World

level = lle.cooperation_level(World.level(6))
assert level in CooperationLevel.cooperative_subtypes()
```

The same vocabulary is available on the generators via the optional `profile=` parameter (only meaningful when `cooperative=True`): the generator keeps sampling until the produced world classifies as the requested level.

```python
world = lle.generate(kind="constructive", n_agents=2, n_lasers=1,
                     cooperative=True, profile=CooperationLevel.ASYMMETRIC,
                     seed=0)
assert lle.cooperation_level(world) is CooperationLevel.ASYMMETRIC
```

## Creating custom maps
You can create custom maps in two ways: using a plain string or a TOML file.

### Plain string
In this very simple text-based syntax, every tile is encoded by one or by a few characters (see the encoding in the below table), and tiles are separated by whitespaces. A new row is indicated by a new line.

| Character | Tile | Walkable | Comment |
------------|------|----------|---------|
| `.` | Floor | Yes | The most basic tile. |
| `@` | Wall  | No | A wall that blocks lasers. |
| `X` | Exit  | Yes | An exit tile. The agent can no longer move after reaching it. |
| `G` | Gem   | Yes | A gem to collect. |
| `S<n>` | Start | Yes | Start position of agent `n`. |
| `L<n><d>` | Laser source | No | Source of a laser of colour `n` (a number) beaming toward the direction `d` (N, S, E, W). |
| `V` | Void | Yes | A void tile. The agent dies if it walks on it |

For instance, assuming that a file "map.txt" has the following content
```
 S0 . G . X
 S1 @ . . .
L0E . . V V
 @  @ . V V
 G  . . . X
```
Then, the below code sample generates the world shown below.
```python
with open("map.txt") as f:
  str_map = f.read()
# Alternative: World.from_file("map.txt")
world = World(str_map)
img = world.get_image()
plt.imshow(img)
plt.show()
```
![pdoc logo](../../docs/example_custom.png)

### Toml syntax
You can also use TOML files to define maps, which enables more complex maps, for instance with random start positions. The below TOML file shows an example of a map with random start positions.

Positions can be specified as a list of positions `{i, j}` and rectangles `{i_min, i_max, j_min, j_max}`, and the `world_string` field can be used to define the map as discussed in the "Plain string" section.

```toml
# Schema for autocompletion and type checking
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
LLE has received the best paper award at at [BNAIC 2023](https://bnaic2023.tudelft.nl/static/media/BNAICBENELEARN_2023_paper_124.c9f5d29e757e5ee27c44.pdf):
```bibtex
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
from .observations import ObservationType
from .types import AgentId, LaserId, Position
from .world import Action, EventType, World, WorldEvent, WorldState
from .solver import CooperationLevel, cooperation_level, is_cooperative, solve
from .generator import generate

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
    "is_cooperative",
    "cooperation_level",
    "CooperationLevel",
    "generate",
    "make_pool",
]
