"""Fine-grained cooperation analysis of LLE trajectories.

This module turns a trajectory (a sequence of joint actions) into a *temporal
helper graph* and extracts structural cooperation properties from it.


See `analyse_cooperation` for the dependency definition, `TemporalDependencyGraph`
for the graph queries, and `CooperationProfile` for the summary of properties.
"""

from .analyser import detect_dependencies, profile_plan
from .graph import DependencyEdge, TemporalDependencyGraph
from .profile import PlanProfile

__all__ = [
    "profile_plan",
    "detect_dependencies",
    "DependencyEdge",
    "TemporalDependencyGraph",
    "PlanProfile",
]
