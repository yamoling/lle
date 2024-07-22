from typing import Optional, final
from abc import ABC
from .direction import Direction
from .types import AgentId, LaserId, Position

class Tile(ABC):
    @property
    def pos(self) -> Position:
        """The (i, j) position of the tile."""
    def __str__(self) -> str: ...
    def __repr__(self) -> str: ...
    def __eq__(self, other: object) -> bool: ...

@final
class Gem(Tile):
    @property
    def agent(self) -> Optional[AgentId]:
        """The id of the agent currently standing on the tile, if any."""
    @property
    def is_collected(self) -> bool:
        """Whether the gem has been collected or not."""

@final
class Laser(Tile):
    @property
    def agent(self) -> Optional[AgentId]:
        """The id of the agent currently standing on the tile, if any."""

    @property
    def is_on(self) -> bool:
        """Whether the laser is on."""

    @property
    def is_off(self) -> bool:
        """Whether the laser is off."""

    @property
    def agent_id(self) -> AgentId:
        """The id of the agent that can block the laser."""
    @property
    def direction(self) -> Direction:
        """The direction of the laser beam."""
    @property
    def laser_id(self) -> LaserId:
        """The ID of the laser."""

    @property
    def is_enabled(self) -> bool:
        """Whether the laser is enabled."""

    @property
    def is_disabled(self) -> bool:
        """Whether the laser is disabled."""

@final
class LaserSource(Tile):
    agent_id: AgentId
    """The id (colour) of the agent that can block the laser."""

    @property
    def direction(self) -> Direction:
        """
        The direction of the laser beam.
        The direction can currently not be changed after creation of the `World`.
        """

    @property
    def laser_id(self) -> LaserId:
        """The ID of the laser."""

    def enable(self):
        """Enable the laser."""
    def disable(self):
        """Disable the laser."""

    def set_colour(self, agent_id: AgentId):
        """Change the colour of the laser to the one of the given agent ID."""
