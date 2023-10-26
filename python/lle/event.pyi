from enum import Enum
from typing import final

from .types import AgentId

@final
class EventType(Enum):
    AGENT_EXIT: EventType
    GEM_COLLECTED: EventType
    AGENT_DIED: EventType

@final
class WorldEvent:
    agent_id: AgentId
    event_type: EventType

    def __str__(self): ...
    def __repr__(self): ...
