from typing import Optional
from lle import AgentId
from .direction import Direction

class Tile:
    agent: Optional[int]

class Gem(Tile):
    is_collected: bool

class Laser(Tile):
    is_on: bool
    agent_id: AgentId
    direction: Direction

class LaserSource(Tile):
    agent_id: AgentId
    direction: Direction

    @property
    def is_on(self) -> bool: ...
    @property
    def is_off(self) -> bool: ...
