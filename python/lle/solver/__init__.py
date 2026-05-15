"""Public solver API: lle.solve and lle.is_cooperative.

pysat is required at call time; absence raises ImportError pointing at the
[generator] extra.
"""

from __future__ import annotations

from lle import Action, World

_PYSAT_HINT = (
    "lle.solve / lle.is_cooperative require the 'generator' extra. Install with: pip install laser-learning-environment[generator]"
)


def _require_pysat() -> None:
    try:
        import pysat.solvers  # noqa: F401
    except ImportError as exc:
        raise ImportError(_PYSAT_HINT) from exc


def _default_t_max(world: World) -> int:
    return (world.width * world.height) // 2


def solve(world: World, t_max: int | None = None) -> list[tuple[Action, ...]] | None:
    """Find a joint plan reaching all exits within `t_max` steps.

    Returns a list of length `t_max`, each entry a tuple of `Action`
    of length `world.n_agents`. Trailing rows after exit are STAY.
    Returns None if no plan exists within `t_max`.
    """
    _require_pysat()
    from .world_solver import WorldSolver  # local import for ImportError gate

    t = _default_t_max(world) if t_max is None else t_max
    solver = WorldSolver(world, t_max=t)
    sat, model = solver.solve()
    if not sat or model is None:
        return None
    return solver.extract_plan(model)


def is_cooperative(world: World, t_max: int | None = None) -> bool:
    """True iff the world is solvable under standard semantics AND UNSAT
    under strict-laser semantics."""
    _require_pysat()
    from .world_solver import LaserMode, WorldSolver

    t = _default_t_max(world) if t_max is None else t_max
    standard_sat, _ = WorldSolver(world, t_max=t, laser_mode=LaserMode.STANDARD).solve()
    if not standard_sat:
        return False
    strict_sat, _ = WorldSolver(world, t_max=t, laser_mode=LaserMode.STRICT).solve()
    return not bool(strict_sat)


__all__ = ["solve", "is_cooperative"]
