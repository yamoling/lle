"""
# The Laser Learning Environment
The Laser Learning Environment (LLE) is a multi-agent gridworld aimed for
research in cooperative multi-agent reinforcement learning. The main dynamic of LLE revolves around lasers since walking in a laser is deadly for all agents except those of the same colour as the laser. When an agent of the same colour as the laser walks in a laser, the laser is blocked and other agents can safely walk through it.

LLE has diﬀerent types of tiles: floor, start, wall, laser, laser source, void, gem, and exit tiles, as illustrated in the below image.

![pdoc logo](../../docs/lvl6-annotated.png)

## Levels of control
LLE can be used at two levels of control:
  - As a general-purpose gridworld (`World` class)
  - As an RL environment (`LLE` class)

### Low-level with `World`
This notebook goes through the `World` class and how to use it. The `World` class is meant to be used for low-level control of LLE, as opposed to the `LLE` class, meant for generic high-level control in multi-agent reinforcement learning.

```python
from lle import World, Action

# A map with one agent, one gem and one exit.
world = World("S0 . G X")
world.reset()
state = world.get_state()
world.step([Action.EAST])
world.set_state(state)
```

### High-level with `LLE`
The `LLE` class (or core) is meant to be used with the `multi-agent-rlenv` library. As such, it encapsulates the `World` class and provides a high-level API for multi-agent reinforcement learning. `LLE` can be used with either in single-objective or multi-objective mode as shown below.

```python
from lle import LLE

env = LLE.level(6).obs_type("layered").single_objective()
obs = env.reset()
action = env.action_space.sample(env.available_actions())
env.step(action)
```

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
