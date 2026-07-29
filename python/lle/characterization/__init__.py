"""Prove, via SAT/UNSAT, which agent dependencies a world *requires*.

`characterize(world, t_max)` returns a `WorldCharacterizer` that answers, for
every plan of length ≤ `t_max`, whether the world demands cooperation (one agent
must block a laser for another) or *mutual* cooperation (every agent both helps
and is helped). `is_cooperative(world)` is the common-case shortcut.

Finer profiles describe the shape of that dependency: `is_convergent(world, k=2)` asks whether one
beneficiary must be helped by `k` distinct agents, and its outgoing dual `is_divergent(world, k=2)`
asks whether one helper must help `k` distinct agents.
"""

from typing import Literal

from ..world import World
from .monotone_cache import MonotoneCache
from .plan import PlanProfile, profile_plan
from .world_characterization import WorldCharacterizer

__all__ = [
    "characterize",
    "is_cooperative",
    "is_asymmetric",
    "is_mutual",
    "is_chained",
    "is_convergent",
    "is_divergent",
    "WorldCharacterizer",
    "MonotoneCache",
    "PlanProfile",
    "profile_plan",
]


def characterize(world: World, t_max: int | Literal["auto"] = "auto"):
    """Characterize `world` over every plan of length ≤ `t_max`.

    Returns a `WorldCharacterizer` exposing properties such as `is_cooperative`
    and `is_mutual`. `t_max` defaults to`(width * height) // 2`.
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
    return w.is_cooperative()


def is_asymmetric(world: World, t_max: int | Literal["auto"] = "auto"):
    """
    Return `True` if the provided world requires asymmetric cooperation to be solved
    in `t_max` steps, i.e. when every solution within `t_max` contains at least one
    help edge whose helper is never helped by another agent.
    """
    w = characterize(world, t_max)
    return w.is_asymmetric()


def is_mutual(world: World, t_max: int | Literal["auto"] = "auto"):
    """
    Return `True` if every solution within `t_max` requires a pair of agents to help
    each other, forming a temporal cycle of order 2.
    """
    w = characterize(world, t_max)
    return w.is_interdependent(2)


def is_chained(world: World, t_max: int | Literal["auto"] = "auto", length: int = 2):
    """
    Return `True` if the provided world requires chained cooperation of at least `length`
    help edges to be solved in `t_max` steps, i.e. when every solution within `t_max`
    exhibits a chain of length >= `length`.
    """
    w = characterize(world, t_max)
    return w.is_chained(length)


def is_convergent(world: World, t_max: int | Literal["auto"] = "auto", k: int = 2):
    """Return whether every solution gives one beneficiary at least `k` distinct helpers."""
    w = characterize(world, t_max)
    return w.is_convergent(k)


def is_divergent(world: World, t_max: int | Literal["auto"] = "auto", k: int = 2):
    """Return whether every solution gives one helper at least `k` distinct beneficiaries."""
    w = characterize(world, t_max)
    return w.is_divergent(k)
