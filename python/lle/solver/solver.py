"""Incremental SAT solver that builds constraints incrementally for time-bounded solving."""

from typing import Literal, overload

from pysat.solvers import Minisat22

from ..world import Action, World
from .constraints import ClauseGenerator, SolveMode
from .types import SolveModeLiteral


@overload
def solve(
    world: World,
    t_max: int | Literal["auto"] = "auto",
    /,
    *,
    mode: SolveModeLiteral | SolveMode = "standard",
) -> list[tuple[Action, ...]] | None: ...


@overload
def solve(
    world: World,
    t_min: int,
    t_max: int | Literal["auto"] = "auto",
    /,
    *,
    mode: SolveModeLiteral | SolveMode = "standard",
) -> list[tuple[Action, ...]] | None: ...


def solve(world: World, /, *min_max, mode: SolveModeLiteral | SolveMode = "standard"):
    """
    Find the shortest plan within the time range [t_min, t_max] (both ends included).

    # Arguments:
        - `t_min`: The minimum time step to consider.
        - `t_max`: The maximum time step to consider. Defaults to (width * height) // 2.
        - `mode`: The solving mode. Check the `SolveMode` enum for more information.
    """
    match min_max:
        case ():
            return _solve(world, 0, "auto", mode=mode)
        case (t_max,):
            return _solve(world, 0, t_max, mode=mode)
        case (t_min, t_max):
            return _solve(world, t_min, t_max, mode=mode)
        case _:
            raise ValueError(f"Invalid arguments: (world, {min_max})")


def _solve(
    world: World,
    t_min: int,
    t_max: int | Literal["auto"],
    *,
    mode: SolveModeLiteral | SolveMode,
) -> list[tuple[Action, ...]] | None:
    if t_max == "auto":
        t_max = (world.width * world.height) // 2
    # ClauseGenerator accepts both SolveMode objects and raw strings (including
    # parameterised modes like "no-interdependence-N" that have no PySolveMode variant).
    gen = ClauseGenerator(world, t_max, mode)
    t_min = max(gen.solution_lower_bound, t_min)
    if t_min > t_max:
        return None
    for t in range(t_min, t_max + 1):
        clauses, assumptions = gen.generate(t)
        model = solve_model(clauses, assumptions=assumptions)
        if model is not None:
            return _to_plan(gen.decode_plan(model, t))
    return None


def solve_model(clauses: list[list[int]], *, assumptions: list[int] | None = None) -> list[int] | None:
    """
    Solve the SAT problem with the given clauses and assumptions, returning the literals' values
    if a solution is found, or `None` if the clauses are unsatisfiable.
    """
    if assumptions is None:
        assumptions = []
    with Minisat22(bootstrap_with=clauses) as solver:
        if solver.solve(assumptions=assumptions):
            model = solver.get_model()
            assert model is not None
            return model


def _to_plan(joint_actions: list[list[Action]]) -> list[tuple[Action, ...]]:
    return [tuple(joint) for joint in joint_actions]
