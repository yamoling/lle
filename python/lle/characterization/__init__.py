"""Prove, via SAT/UNSAT, which agent dependencies a world *requires*.

`characterize(world, t_max)` returns a `WorldCharacterizer` that answers, for
every plan of length ≤ `t_max`, whether the world demands cooperation (one agent
must block a laser for another) or *mutual* cooperation (every agent both helps
and is helped). `is_cooperative(world)` is the common-case shortcut.
"""

from typing import Literal

from ..world import World
from .trajectory import TrajectoryProfile
from .world_characterization import WorldCharacterizer

__all__ = ["characterize", "is_cooperative", "WorldCharacterizer", "TrajectoryProfile"]


def characterize(world: World, t_max: int | Literal["auto"] = "auto"):
    """Characterize `world` over every plan of length ≤ `t_max`.

    Returns a `WorldCharacterizer` exposing properties such as `is_cooperative`
    and `is_mutual`. `t_max` defaults to ``(width * height) // 2``.
    """
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


def is_mutual(world: World, t_max: int | Literal["auto"] = "auto"):
    """
    Return `True` if the provided world requires mutual cooperation to be solved
    in `t_max` steps, i.e. when there exists a solution with laser blocking enabled
    but not without laser blocking.
    """
    w = characterize(world, t_max)
    return w.is_mutual


def is_chained(world: World, t_max: int | Literal["auto"] = "auto"):
    """
    Return `True` if the provided world requires chained cooperation to be solved
    in `t_max` steps, i.e. when there exists a solution with laser blocking enabled
    but not without laser blocking.
    """
    w = characterize(world, t_max)
    return w.is_chained
