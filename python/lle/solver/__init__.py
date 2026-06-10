"""World solving helpers.

Use `solve` to search for a shortest joint plan, and `is_cooperative` to test
whether a level requires laser blocking. The `mode` parameter on `solve` controls
cooperation semantics:

- `"standard"` (default): agents may cooperate freely.
- `"no-cooperation"`: no non-owner agent may enter any laser span.
- `"no-mutual-cooperation"`: no pair of agents may mutually help each other.

These functions need the optional `generator` extra at runtime because they
rely on the SAT solver backend.
"""

from __future__ import annotations

from typing import Literal

from ..world import World
from .constraints import SolveMode
from .solver import (
    solve,
    solve_model,
)
from .types import SolveModeLiteral


def is_cooperative(world: World, t_max: int | Literal["auto"] = "auto"):
    """
    Return `True` if the provided world requires cooperation to be solved
    in `t_max` steps, i.e. when there exists a solution with laser blocking enabled
    but not without laser blocking.
    """
    standard_plan = solve(world, t_max)
    if standard_plan is None:
        return False
    strict_plan = solve(world, t_max, mode="no-cooperation")
    return strict_plan is None


__all__ = ["is_cooperative", "solve", "solve_model", "SolveMode", "SolveModeLiteral", "SolveMode"]
