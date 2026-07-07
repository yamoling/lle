from typing import Literal

from ..world import World
from .trajectory import TrajectoryProfile
from .world_characterization import WorldCharacterizer

__all__ = ["TrajectoryProfile"]


def characterize(world: World, t_max: int | Literal["auto"] = "auto"):
    if t_max == "auto":
        t_max = (world.width * world.height) // 2
    return WorldCharacterizer(world, t_max)


def is_cooperative(world: World, t_max: int | Literal["auto"] = "auto"):
    """
    Return `True` if the provided world requires cooperation to be solved
    in `t_max` steps, i.e. when there exists a solution with laser blocking enabled
    but not without laser blocking.
    """
    w = characterize(world, t_max)
    return w.is_cooperative
