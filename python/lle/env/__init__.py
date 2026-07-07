"""High-level environment builder and MARL integration helpers.

The `LLE` environment wraps a `World` and exposes the interface expected by
`marlenv`. Use `Builder` to configure observations, rewards, extras, and death
handling before calling `build()`.
"""

from .builder import Builder
from .env import LLE
from .extras_generators import ExtraGenerator, LaserSubgoal, NoExtras
from .reward_strategy import MultiObjective, RewardStrategy, SingleObjective

__all__ = [
    "LLE",
    "Builder",
    "RewardStrategy",
    "SingleObjective",
    "MultiObjective",
    "ExtraGenerator",
    "NoExtras",
    "LaserSubgoal",
]
