"""Environment components that do not depend on the physics of the world."""

from enum import IntEnum
from typing import Literal


class DeathStrategy(IntEnum):
    END = 0
    """End the game when an agent dies"""
    RESPAWN = 1
    """The agent respawns on its start position when it dies"""

    @staticmethod
    def from_str(value: Literal["end", "respawn"]):
        match value:
            case "end":
                return DeathStrategy.END
            case "respawn":
                return DeathStrategy.RESPAWN
        raise ValueError(f"Unknown death strategy: {value}")
