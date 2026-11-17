"""Observation types of the `CLLE` environment."""

from enum import Enum
from typing import Literal

ObservationTypeLiteral = Literal["state"]


class ObservationType(str, Enum):
    """The observation (and state) types of `CLLE`."""

    STATE = "state"
    """
    A flat `float32` vector, computed by `ContinuousWorld.observation` (per agent, ego-first with
    relative coordinates) or `ContinuousWorld.state_vector` (the state, in absolute coordinates).
    """

    @staticmethod
    def from_str(s: ObservationTypeLiteral | str) -> "ObservationType":
        """
        The observation type named `s`.

        Raises a `ValueError` if no observation type has this name.

        @ai-generated
        """
        return ObservationType(s)
