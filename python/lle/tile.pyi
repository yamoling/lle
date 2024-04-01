from typing import Optional, final
from .direction import Direction
from .types import AgentId, LaserId

@final
class Gem:
    agent: Optional[AgentId]
    """The id of the agent currently standing on the tile, if any."""
    is_collected: bool
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
    """The direction of the laser beam.."""
    def set_agent_id(self, agent_id: AgentId):
        """Change the 'colour' of the laser to the one of the agent given as argument."""
    def turn_on(self):
        """Turn the laser on."""
    def turn_off(self):
        """Turn the laser off."""
    def __str__(self) -> str: ...
    def __repr__(self) -> str: ...
