"""Fine-grained cooperation analysis of LLE trajectories.

This module turns a trajectory (a sequence of joint actions) into a *temporal
helper graph* and extracts structural cooperation properties from it.


See `analyse_cooperation` for the dependency definition, `TemporalDependencyGraph`
for the graph queries, and `CooperationProfile` for the summary of properties.
"""

from .analyser import detect_dependencies, profile_trajectory
from .graph import DependencyEdge, TemporalDependencyGraph
from .profile import TrajectoryProfile

__all__ = [
    "profile_trajectory",
    "detect_dependencies",
    "DependencyEdge",
    "TemporalDependencyGraph",
    "TrajectoryProfile",
]
