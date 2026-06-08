"""Incremental SAT solver that builds constraints incrementally for time-bounded solving."""

from typing import Literal, overload

from pysat.solvers import Minisat22

from ..world import Action, World
from .constraints import ClauseGenerator


@overload
def solve(world: World, t_max: int | Literal["auto"] = "auto", /) -> list[tuple[Action, ...]] | None: ...
@overload
def solve(world: World, t_min: int, t_max: int | Literal["auto"] = "auto", /) -> list[tuple[Action, ...]] | None: ...


def solve(world: World, *min_max):
    """
    Find the shortest plan within the time range [t_min, t_max] (both ends included).

    # Arguments:
        - `t_min`: The minimum time step to consider.
        - `t_max`: The maximum time step to consider. Defaults to (width * height) // 2.
    """
    match min_max:
        case ():
            return _solve(world, 0, "auto")
        case (t_max,):
            return _solve(world, 0, t_max)
        case (t_min, t_max):
            return _solve(world, t_min, t_max)
        case _:
            raise ValueError(f"Invalid arguments: (world, {min_max})")


def _solve(world: World, t_min: int, t_max: int | Literal["auto"]) -> list[tuple[Action, ...]] | None:
    if t_max == "auto":
        t_max = (world.width * world.height) // 2
    gen = ClauseGenerator(world, t_max)
    t_min = max(gen.solution_lower_bound, t_min)
    if t_min > t_max:
        return None

    # Generate the clauses for t in [0, t_min)
    clauses = [clause for t in range(t_min) for clause in gen.generate(t)]
    for t in range(t_min, t_max + 1):
        clauses.extend(gen.generate(t))
        with Minisat22(bootstrap_with=clauses) as solver:
            solver.append_formula(gen.objective(t))
            if solver.solve():
                model = solver.get_model()
                assert model is not None
                return _to_plan(gen.decode_plan(model, t))
    return None


def solve_no_cooperation(
    world: World,
    t_min: int = 0,
    t_max: int | Literal["auto"] = "auto",
) -> list[tuple[Action, ...]] | None:
    """Find the shortest plan that requires no laser blocking (no cooperation).

    Returns ``None`` when every valid plan within ``[t_min, t_max]`` requires at
    least one blocking event, i.e. cooperation is *strictly required* in that range.
    """
    if t_max == "auto":
        t_max = (world.width * world.height) // 2
    gen = ClauseGenerator(world, t_max)
    t_min = max(gen.solution_lower_bound, t_min)
    if t_min > t_max:
        return None

    # Pre-generate no-blocking unit clauses for the entire horizon so they are
    # present from the very first solver instance onward.
    clauses = [clause for t in range(t_min) for clause in gen.generate(t)] + [
        c for t in range(t_max + 1) for c in gen.no_blocking_clauses(t)
    ]
    for t in range(t_min, t_max + 1):
        clauses.extend(gen.generate(t))
        with Minisat22(bootstrap_with=clauses) as solver:
            solver.append_formula(gen.objective(t))
            if solver.solve():
                model = solver.get_model()
                assert model is not None
                return _to_plan(gen.decode_plan(model, t))
    return None


def _to_plan(joint_actions: list[list[Action]]) -> list[tuple[Action, ...]]:
    return [tuple(joint) for joint in joint_actions]
