from typing import final, Dict, Set

from .types import AgentId, Position
from .world import World
from .direction import Direction

@final
class WorldBuilder:
    def __init__(self, width: int, height: int, n_agents: int): ...
    def build(self) -> World:
        """Build a world with the given parameters."""
    @property
    def start_positions(self) -> Dict[AgentId, Position]: ...
    @property
    def exit_positions(self) -> Set[Position]: ...
    @property
    def width(self) -> int: ...
    @property
    def height(self) -> int: ...
    @property
    def n_agents(self) -> int: ...
    @property
    def available_positions(self) -> Set[Position]: ...
    def set_start(self, pos: Position, agent_id: AgentId): ...
    def add_exit(self, pos: Position): ...
    def add_wall(self, pos: Position): ...
    def add_gem(self, pos: Position): ...
    def add_laser_source(self, pos: Position, agent_id: AgentId, direction: Direction): ...
    def clear(self, pos: Position):
        """Clear the tile at the given position and turn it back to a floor."""
    def world_str(self) -> str:
        """Return the current string representation of the world."""
    def reset(self):
        """Reset the builder to an empty world."""
    def can_build(self) -> bool:
        """
        Check if the world can be built with the current configuration.

        A `World` can be built if there are enough start and exits tiles for all the agents.
        """
