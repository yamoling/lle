from typing import Optional, final
from .direction import Direction
from .types import AgentId

class Tile:
    agent: Optional[int]

@final
class Gem(Tile):
    is_collected: bool
    def __str__(self) -> str: ...
    def __repr__(self) -> str: ...

@final
class Laser(Tile):
    is_on: bool
    is_off: bool
    agent_id: AgentId
    direction: Direction
    def __str__(self) -> str: ...
    def __repr__(self) -> str: ...

@final
class LaserSource(Tile):
    agent_id: AgentId
    direction: Direction
    def __str__(self) -> str: ...
    def __repr__(self) -> str: ...
