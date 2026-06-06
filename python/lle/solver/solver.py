"""Incremental SAT solver that builds constraints incrementally for time-bounded solving."""

from typing import Literal

from pysat.solvers import Minisat22

from ..world import Action, World
from .constraints import (
    ConstraintContext,
    ConstraintGenerator,
    CooperationConstraints,
    InitializationConstraints,
    LaserConstraints,
    MovementConstraints,
    ObjectiveGenerator,
)
from .variable_factory import VariableFactory


def solve(world: World, t_min: int = 0, t_max: int | Literal["auto"] = "auto") -> list[tuple[Action, ...]] | None:
    """
    Find the shortest plan within the time range [t_min, t_max] (both ends included).

    Arguments:
    ---------
        - `t_min`: The minimum time step to consider.
        - `t_max`: The maximum time step to consider. Defaults to (width * height) // 2.
    """
    if t_max == "auto":
        t_max = (world.width * world.height) // 2
    ctx = ConstraintContext(world, t_max)
    t_min = max(ctx.solution_lower_bound, t_min)
    if t_min > t_max:
        return None

    # Constraint generators
    var = VariableFactory()
    generators: list[ConstraintGenerator] = [
        InitializationConstraints(var, ctx),
        MovementConstraints(var, ctx),
        LaserConstraints(var, ctx),
    ]
    objective = ObjectiveGenerator(var, ctx)
    # Generate initial clauses for t in [0, t_min)
    clauses = [clause for generator in generators for t in range(t_min) for clause in generator.generate(t)]
    for t in range(t_min, t_max + 1):
        clauses.extend([clause for generator in generators for clause in generator.generate(t)])
        with Minisat22(bootstrap_with=clauses) as solver:
            solver.append_formula(objective.generate(t))
            if solver.solve():
                model = solver.get_model()
                assert model is not None
                plan = extract_plan(var, model, t)
                return plan
    return None


def solve_no_cooperation(
    world: World,
    t_min: int = 0,
    t_max: int | Literal["auto"] = "auto",
) -> list[tuple[Action, ...]] | None:
    """Find the shortest plan that requires no laser blocking (no cooperation).

    Functionally equivalent to ``solve(..., laser_mode=LaserMode.STRICT)`` but
    implemented via ``CooperationConstraints.no_blocking_clauses`` rather than a
    separate constraint generator.  Returns ``None`` when every valid plan within
    ``[t_min, t_max]`` requires at least one blocking event, i.e. cooperation is
    *strictly required* in that range.
    """
    if t_max == "auto":
        t_max = (world.width * world.height) // 2
    ctx = ConstraintContext(world, t_max)
    t_min = max(ctx.solution_lower_bound, t_min)
    if t_min > t_max:
        return None

    var = VariableFactory()
    coop = CooperationConstraints(var, ctx)
    generators: list[ConstraintGenerator] = [
        InitializationConstraints(var, ctx),
        MovementConstraints(var, ctx),
        LaserConstraints(var, ctx),
    ]
    objective = ObjectiveGenerator(var, ctx)

    # Pre-generate no-blocking unit clauses for the entire horizon so they are
    # present from the very first solver instance onward.
    no_blocking: list[list[int]] = [c for t in range(t_max + 1) for c in coop.no_blocking_clauses(t)]

    clauses = [clause for gen in generators for t in range(t_min) for clause in gen.generate(t)] + no_blocking
    for t in range(t_min, t_max + 1):
        clauses.extend([clause for gen in generators for clause in gen.generate(t)])
        with Minisat22(bootstrap_with=clauses) as solver:
            solver.append_formula(objective.generate(t))
            if solver.solve():
                model = solver.get_model()
                assert model is not None
                return extract_plan(var, model, t)
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
            _, color, x, y, t = obj
            positions.setdefault(color, {})[t] = (x, y)
        elif obj[0] == "done":
            _, t = obj
            done_times.append(t)
    agent_colors = sorted(positions.keys())
    plan: list[tuple[Action, ...]] = []
    for t in range(t_end):
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
