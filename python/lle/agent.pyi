from typing import final
from .types import AgentId

@final
class Agent:
    @property
    def num(self) -> AgentId:
        """The agent ID."""
    @property
    def has_arrived(self) -> bool:
        """Whether the agent has arrived at the exit."""
    @property
    def is_dead(self) -> bool:
        """Whether the agent is dead."""
    @property
    def is_alive(self) -> bool:
        """Whether the agent is alive."""
