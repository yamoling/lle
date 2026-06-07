"""World solving and cooperation analysis helpers.

Use `solve` to search for a shortest joint plan, `solve_hybrid` to search for
a shortest joint plan via incremental SAT reuse, `solve_sat` to search for a
fixed-horizon plan of length exactly `t_max`, `is_cooperative` to test whether
a level requires laser blocking, and `cooperation_level` to obtain the more
precise structural classification.

These functions need the optional `generator` extra at runtime because they
rely on the SAT solver backend.
"""

from __future__ import annotations

from typing import Literal

from ..world import World
from .constraints import CooperationConstraints
from .solver import solve, solve_no_cooperation


def is_cooperative(world: World, t_max: int | Literal["auto"] = "auto"):
    """
    Return `True` if the provided world requires cooperation to be solved
    in `t_max` steps, i.e. when there exist a solution with laser blocking enabled (`LaserMode.STANDARD`)
    but not with lasers can not be blocked (`LaserMode.STRICT`).
    """
    standard_plan = solve(world, t_max)
    if standard_plan is None:
        return False
    strict_plan = solve_no_cooperation(world, t_max=t_max)
    return strict_plan is None


__all__ = [
    "CooperationConstraints",
    "is_cooperative",
    "solve",
    "solve_no_cooperation",
]
