"""
The Laser Learning Environment (LLE) is a multi-agent gridworld aimed for
research in cooperative multi-agent reinforcement learning.

LLE can be used at two levels of control:
    - As an RL environment (`LLE` class)
    - As a multi-purpose gridworld (`World` class)

```python
from lle import LLE, World

# A map with one agent, one gem and one exit.
world = World("S0 . G X")
world.reset()
state = world.get_state()

# Predefined level 6 geared towards single-objective learning.
env = LLE.level(6).single_objective()
obs = env.reset()
```
"""

__all__ = [
    "__version__",
    "Action",
    "WorldState",
    "Agent",
    "tiles",
    "Direction",
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
    Direction,
    WorldBuilder,
    tiles,
    exceptions,
)

from .observations import ObservationType
from .env import LLE
from . import env
from . import observations
