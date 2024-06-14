from typing import Optional, final
from .direction import Direction
from .types import AgentId, LaserId

@final
class Gem:
    @property
    def agent(self) -> Optional[AgentId]:
        """The id of the agent currently standing on the tile, if any."""
    @property
    def is_collected(self) -> bool:
        """Whether the gem has been collected or not."""
    def __str__(self) -> str: ...
    def __repr__(self) -> str: ...

@final
class Laser:
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
    def __str__(self) -> str: ...
    def __repr__(self) -> str: ...

@final
class LaserSource:
    @property
    def agent_id(self) -> AgentId:
        """The id of the agent that can block the laser."""
    @property
    def laser_id(self) -> LaserId:
        """The ID of the laser."""
    direction: Direction
    """The direction of the laser beam."""
    def __str__(self) -> str: ...
    def __repr__(self) -> str: ...
