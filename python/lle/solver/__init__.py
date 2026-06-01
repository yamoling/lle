"""World solving and cooperation analysis helpers.

Use `solve` to search for a joint plan, `is_cooperative` to test whether a
level requires laser blocking, and `cooperation_level` to obtain the more
precise structural classification. See `CooperationLevel` for the full
classification vocabulary.

These functions need the optional `generator` extra at runtime because they
rely on the SAT solver backend.
"""

from __future__ import annotations

from typing import Literal, Sequence

from ..world import Action, World
from .cooperation_level import CooperationLevel, CooperationLevelStr


def _require_pysat() -> None:
    try:
        import pysat.solvers  # pyright: ignore[reportMissingImports] # noqa: F401
    except ImportError as exc:
        import inspect

        current_frame = inspect.currentframe()
        caller_frame = inspect.getouterframes(current_frame, 2)
        caller_name = caller_frame[1].function
        error_message = f"""The {caller_name!r} function requires the 'pysat' package which is not installed.
To use {caller_name!r}, install lle with: pip install laser-learning-environment[generator]"""
        raise ImportError(error_message) from exc


def _default_t_max(world: World) -> int:
    return (world.width * world.height) // 2


def solve(world: World, t_max: int | Literal["auto"] = "auto"):
    """Find a joint of length `t_max` plan that brings every agent to an exit.

    Returns `None` if no plan exists within the time bound.
    """
    _require_pysat()
    from .world_solver import WorldSolver  # local import for ImportError gate

    t = _default_t_max(world) if t_max == "auto" else t_max
    solver = WorldSolver(world, t_max=t)
    sat, model = solver.solve()
    if not sat or model is None:
        return None
    return solver.extract_plan(model)


def is_cooperative(world: World, t_max: int | Literal["auto"] = "auto"):
    """
    Return `True` if the provided world requires cooperation to be solved
    in `t_max` steps, i.e. when there exist a solution with laser blocking enabled (`LaserMode.STANDARD`)
    but not with lasers can not be blocked (`LaserMode.STRICT`).
    """
    _require_pysat()
    from .world_solver import LaserMode, WorldSolver

    t = _default_t_max(world) if t_max == "auto" else t_max
    standard_sat, _ = WorldSolver(world, t_max=t, laser_mode=LaserMode.STANDARD).solve()
    if not standard_sat:
        return False
    strict_sat, _ = WorldSolver(world, t_max=t, laser_mode=LaserMode.STRICT).solve()
    return not bool(strict_sat)


def cooperation_level(world: World, t_max: int | Literal["auto"] = "auto"):
    """Return the precise cooperation classification for `world`.

    See `CooperationLevel` for the meaning of each member.
    """
    _require_pysat()
    from .profile_analyzer import classify

    t = _default_t_max(world) if t_max == "auto" else t_max
    return classify(world, t)


def cooperation_level_trajectory(world: World, trajectory: Sequence[tuple[Action, ...]]):
    """Return the cooperation classification induced by an explicit trajectory."""
    _require_pysat()
    from .profile_analyzer import classify

    return classify(world, trajectory)


__all__ = [
    "CooperationLevel",
    "cooperation_level",
    "cooperation_level_trajectory",
    "is_cooperative",
    "solve",
    "CooperationLevelStr",
]
