"""Observation types of the `CLLE` environment."""

from enum import Enum
from typing import Literal

ObservationTypeLiteral = Literal["state", "layered", "lidar", "rgb", "rgb-image"]


class ObservationType(str, Enum):
    """The observation (and state) types of `CLLE`.

    Their values follow `lle.ObservationType` where both exist, and `"rgb"` is an alias of
    `"rgb-image"`.
    """

    STATE = "state"
    """
    A flat `float32` vector, computed by `ContinuousWorld.observation` (per agent, ego-first with
    relative coordinates) or `ContinuousWorld.state_vector` (the state, in absolute coordinates),
    optionally followed by the static features (`Builder.include_static_features`).
    """
    LAYERED = "layered"
    """
    The rasterized map, of shape `(C, height * res, width * res)`, computed by
    `ContinuousWorld.layered` (`Builder.layered_resolution` sets `res`). Every agent receives the
    same array.
    """
    LIDAR = "lidar"
    """
    Per-agent ray casts, followed by the agent's own `STATE` block, computed by
    `ContinuousWorld.lidar` (`Builder.lidar` sets the number of rays and their range). It is not
    a valid state type.
    """
    RGB_IMAGE = "rgb-image"
    """
    The rendered frame, of shape `(height * 32, width * 32, 3)`, as `float32` like
    `lle.ObservationType.RGB_IMAGE`. Every agent receives the same frame.
    """

    @classmethod
    def _missing_(cls, value: object):
        """
        Accept the alias `"rgb"` of `RGB_IMAGE`.

        @ai-generated
        """
        if value == "rgb":
            return cls.RGB_IMAGE
        return None

    @staticmethod
    def from_str(s: ObservationTypeLiteral | str) -> "ObservationType":
        """
        The observation type named `s` (`"rgb"` is an alias of `"rgb-image"`).

        Raises a `ValueError` if no observation type has this name.

        @ai-generated
        """
        return ObservationType(s)
