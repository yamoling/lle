"""Fine-grained cooperation analysis for LLE trajectories.

This module turns a trajectory (a sequence of joint actions) into a *temporal
helper graph* and extracts structural cooperation properties from it.

```python
from lle import World
from lle.cooperation import analyse_cooperation

world = World(...)
graph = analyse_cooperation(world, trajectory)
profile = graph.profile()
print(profile.longest_chain, profile.max_fan_out)
```

See `analyse_cooperation` for the dependency definition, `TemporalDependencyGraph`
for the graph queries, and `CooperationProfile` for the summary of properties.
"""

from __future__ import annotations

from .analyser import analyse_cooperation, detect_dependencies
from .characterization import WorldCharacterization, characterize
from .graph import DependencyEdge, TemporalDependencyGraph
from .profile import CooperationProfile

__all__ = [
    "analyse_cooperation",
    "detect_dependencies",
    "DependencyEdge",
    "TemporalDependencyGraph",
    "CooperationProfile",
    "characterize",
    "WorldCharacterization",
]
