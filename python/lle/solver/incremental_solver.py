"""Incremental SAT solver that builds constraints incrementally for time-bounded solving."""

from typing import Literal

from pysat.solvers import Minisat22

from ..world import World
from ._constraints import (
    METHOD_LOCAL,
    ConstraintContext,
    InitializationConstraints,
    MovementConstraints,
)
from ._constraints.base import ConstraintGenerator
from .laser_mode import LaserMode
from .variable_factory import VariableFactory


def solve(
    world: World,
    t_min: int = 0,
    t_max: int | Literal["auto"] = "auto",
    *,
    laser_mode: LaserMode = LaserMode.STANDARD,
) -> tuple[bool, list | None]:
    """
    Find the shortest plan within the time range [t_min, t_max] (both ends included).

    Arguments:
    ---------
        - `t_min`: The minimum time step to consider.
        - `t_max`: The maximum time step to consider. Defaults to (width * height) // 2.
    """
    if t_max == "auto":
        t_max = (world.width * world.height) // 2
    ctx = ConstraintContext(world, t_min, t_max)
    t_min = max(ctx.solution_lower_bound, t_min)
    if t_min > t_max:
        return False, None

    # Constraint generators
    var = VariableFactory()
    generators: list[ConstraintGenerator] = [
        InitializationConstraints(var, ctx),
        MovementConstraints(var, ctx),
        laser_mode.get(var, ctx),
    ]
    # Generate initial clauses for t in [0, t_min]
    clauses = [clause for generator in generators for t in range(t_min + 1) for clause in generator.generate(t)]
    with Minisat22(bootstrap_with=clauses) as solver:
        assumptions = []
        for t in range(t_min, t_max + 1):
            # Generate the clauses for the current horizon [t_min, current_t]
            new_clauses = [clause for generator in generators for clause in generator.generate(t)]
            if len(new_clauses) == 0:
                continue
            solver.append_formula(new_clauses)
            done_vars = [0]  # TODO

            # Get the done variable for this specific time step
            # done_vars is indexed from 0 to (current_t - t_min)
            done_t = done_vars[t - t_min]

            # Try to solve with current done_t assumption
            if solver.solve(assumptions=[done_t]):
                model = solver.get_model()
                return True, model

            # If this horizon fails, exclude it from future attempts
            assumptions.append(-done_t)
    return False, None
