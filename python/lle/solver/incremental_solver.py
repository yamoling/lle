"""Incremental SAT solver that builds constraints incrementally for time-bounded solving."""

from typing import Literal

from pysat.solvers import Minisat22

from ..world import Action, World
from ._constraints import (
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
) -> list[tuple[Action, ...]] | None:
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
        return None

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
        # assumptions = []
        for t in range(t_min, t_max + 1):
            # Generate the clauses for the current horizon [t_min, current_t]
            new_clauses = [clause for generator in generators for clause in generator.generate(t)]
            if len(new_clauses) == 0:
                continue
            solver.append_formula(new_clauses)
            # Initialize done variables for each time step
            # done_vars = [var.done(t) for t in range(t_min, t_max + 1)]

            # Get the done variable for this specific time step
            # done_t = done_vars[t - t_min]

            # Try to solve with current done_t assumption
            if solver.solve():  # assumptions=[done_t]):
                model = solver.get_model()
                assert model is not None
                plan = extract_plan(var, model, t)
                return plan

            # If this horizon fails, exclude it from future attempts
            # assumptions.append(-done_t)
    return None


def extract_plan(var: VariableFactory, model: list[int], t_end: int) -> list[tuple[Action, ...]]:
    positions = dict[int, dict[int, tuple[int, int]]]()
    done_times: list[int] = []
    for lit in model:
        if lit <= 0:
            continue
        obj = var.pool.obj(lit)
        if not obj:
            continue
        if obj[0] == "agent":
            _, color, (x, y), t = obj
            positions.setdefault(color, {})[t] = (x, y)
        elif obj[0] == "done":
            _, t = obj
            done_times.append(t)
    agent_colors = sorted(positions.keys())
    plan: list[tuple[Action, ...]] = []
    for t in range(t_end - 1):
        row: list[Action] = []
        for color in agent_colors:
            y1, x1 = positions[color][t]
            y2, x2 = positions[color][t + 1]
            dx, dy = x2 - x1, y2 - y1
            try:
                a = Action.from_delta(dx, dy)
            except ValueError as e:
                raise ValueError(f"Invalid movement for agent {color} at t={t}->{t + 1}") from e
            row.append(a)
        plan.append(tuple(row))
    return plan
