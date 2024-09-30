"""
# The Laser Learning Environment
The Laser Learning Environment (LLE) is a multi-agent gridworld aimed for
research in cooperative multi-agent reinforcement learning. The main dynamic of LLE revolves around lasers since walking in a laser is deadly for all agents except those of the same colour as the laser. When an agent of the same colour as the laser walks in a laser, the laser is blocked and other agents can safely walk through it.

LLE has diﬀerent types of tiles: floor, start, wall, laser, laser source, void, gem, and exit tiles, as illustrated in the below image.

![pdoc logo](../../docs/lvl6-annotated.png)


## General-purpose `World`
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

## Cooperative MARL `LLE`
The `LLE` class is meant for Multi-Agent Reinforcement Leanring (MARL) and to be used with the `multi-agent-rlenv` library. As such, it encapsulates the `World` class and provides a high-level API for multi-agent reinforcement learning. `LLE` can be used with either in single-objective or multi-objective mode as shown below.

```python
from lle import LLE

env = LLE.level(6).obs_type("layered").single_objective()
obs = env.reset()
action = env.action_space.sample(env.available_actions())
env.step(action)
```

## Creating custom maps
You can create custom maps with a simple text-based syntax. Every tile is encoded by one or a few characters (see the encoding in the below table), and tiles are separated by whitespaces. A new row is indicated by a new line.

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

## Citing our work
The environment has been presented at [EWRL 2023](https://openreview.net/pdf?id=IPfdjr4rIs) and at [BNAIC 2023](https://bnaic2023.tudelft.nl/static/media/BNAICBENELEARN_2023_paper_124.c9f5d29e757e5ee27c44.pdf) where it received the best paper award.

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

__all__ = [
    "__version__",
    "Action",
    "WorldState",
    "Agent",
    "tiles",
    "LLE",
    "World",
    "ObservationType",
    "Position",
    "AgentId",
    "LaserId",
    "WorldEvent",
    "EventType",
    "exceptions",
    "WorldBuilder",
    "env",
    "observations",
]

from .types import AgentId, LaserId, Position
from .lle import (
    __version__,
    Action,
    World,
    WorldEvent,
    EventType,
    WorldState,
    Agent,
    WorldBuilder,
    tiles,
    exceptions,
)

from .observations import ObservationType
from .env import LLE
from . import env
from . import observations
