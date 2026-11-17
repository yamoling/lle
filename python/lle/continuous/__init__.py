"""
Continuous LLE (CLLE): the agents are discs that move freely in the plane of an LLE map.

The maps are those of LLE (v1 strings or TOML files, and the built-in levels), without boxes,
lifts, buttons, levers, pressure plates or several layers. Positions are `(x, y)` pairs, with `x`
towards the east (columns) and `y` towards the south (rows), and the centre of the cell `(i, j)`
is `(j + 0.5, i + 0.5)`. An action is an `(ax, ay)` pair in `[-1, 1]²`, in the same order.

- `ContinuousWorld` is the low-level world, the counterpart of `lle.World`.
- `CLLE` is the `marlenv` environment, built with `level`, `from_str` or `from_file`.

```python
import lle.continuous as clle

env = clle.level(6).action_repeat(10).build()
obs, state = env.reset()
done = False
while not done:
    step = env.step(env.sample_action())  # shape (n_agents, 2) in [-1, 1]
    done = step.is_terminal
```
"""

from .world import ContinuousWorld, ContinuousWorldState, PhysicsParams, StepOutcome  # noqa: I001 (native module first)
from .builder import Builder
from .env import CLLE
from .observations import ObservationType, ObservationTypeLiteral
from .reward_strategy import PotentialShapedCLLE

from_str = Builder.from_str
from_file = Builder.from_file
level = Builder.level

__all__ = [
    "CLLE",
    "Builder",
    "ContinuousWorld",
    "ContinuousWorldState",
    "ObservationType",
    "ObservationTypeLiteral",
    "PhysicsParams",
    "PotentialShapedCLLE",
    "StepOutcome",
    "from_file",
    "from_str",
    "level",
]
