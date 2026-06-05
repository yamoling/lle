"""Internal SAT data structures and helper functions.

These helpers are not part of the public API; they support the solver's CNF
construction and model extraction.
"""

from .grid import all_positions, get_neighbours, is_within_bounds
from .model import SATModel
from .types import AgentData, LaserSourceData, Position, agents_from_world, laser_sources_from_world

__all__ = [
    "SATModel",
    "all_positions",
    "get_neighbours",
    "is_within_bounds",
    "AgentData",
    "LaserSourceData",
    "agents_from_world",
    "laser_sources_from_world",
    "Position",
]
