"""Incremental SAT solver that builds constraints incrementally for time-bounded solving."""

from typing import Literal, overload

from pysat.solvers import Minisat22

from ..world import Action, World
from .constraints import ClauseGenerator


@overload
def solve(
    world: World,
    t_max: int | Literal["auto"] = "auto",
    /,
    *,
    allow_cooperation: bool = True,
    raw_assumptions: list[int] = [],
) -> list[tuple[Action, ...]] | None: ...


@overload
def solve(
    world: World,
    t_min: int,
    t_max: int | Literal["auto"] = "auto",
    /,
    *,
    allow_cooperation: bool = True,
    raw_assumptions: list[int] = [],
) -> list[tuple[Action, ...]] | None: ...


def solve(world: World, /, *min_max, raw_assumptions: list[int] | None = None, allow_cooperation: bool = True):
    """
    Find the shortest plan within the time range [t_min, t_max] (both ends included).

    # Arguments:
        - `t_min`: The minimum time step to consider.
        - `t_max`: The maximum time step to consider. Defaults to (width * height) // 2.
        - `allow_cooperation`: Whether to allow cooperation between agents.
        - `raw_assumptions`: A list of raw assumptions to use for solving.
    """
    if raw_assumptions is None:
        raw_assumptions = []
    match min_max:
        case ():
            return _solve(world, 0, "auto", raw_assumptions=raw_assumptions, allow_cooperation=allow_cooperation)
        case (t_max,):
            return _solve(world, 0, t_max, raw_assumptions=raw_assumptions, allow_cooperation=allow_cooperation)
        case (t_min, t_max):
            return _solve(world, t_min, t_max, raw_assumptions=raw_assumptions, allow_cooperation=allow_cooperation)
        case _:
            raise ValueError(f"Invalid arguments: (world, {min_max})")


def _solve(
    world: World,
    t_min: int,
    t_max: int | Literal["auto"],
    *,
    raw_assumptions: list[int],
    allow_cooperation: bool,
) -> list[tuple[Action, ...]] | None:
    if t_max == "auto":
        t_max = (world.width * world.height) // 2
    gen = ClauseGenerator(world, t_max)
    t_min = max(gen.solution_lower_bound, t_min)
    if t_min > t_max:
        return None
    # Generate the clauses for t in [0, t_min)
    clauses = list[list[int]]()
    raw_assumptions = list[int]()
    for t in range(t_min):
        clauses.extend(gen.generate(t))
        if not allow_cooperation:
            raw_assumptions.extend(gen.assume_no_cooperation(t))
    for t in range(t_min, t_max + 1):
        clauses.extend(gen.generate(t))
        if not allow_cooperation:
            raw_assumptions.extend(gen.assume_no_cooperation(t))
        objective = gen.objective(t)
        model = solve_model(clauses + objective, assumptions=raw_assumptions)
        if model is not None:
            return _to_plan(gen.decode_plan(model, t))
    return None


def solve_without_mutual_cooperation(
    world: World,
    t_max: int | Literal["auto"] = "auto",
) -> list[tuple[Action, ...]] | None:
    """
    Find the shortest plan (length in `[solution_lower_bound, t_max]`) in which **no pair of
    agents mutually cooperates**, i.e. there is no pair `{a, b}` such that `a` helps `b` cross
    one of `a`'s laser beams at some point *and* `b` helps `a` likewise at some point.

    Returns the plan, or `None` if every solution of length `<= t_max` requires some pair to
    mutually cooperate (or if the world is unsolvable within `t_max`).
    """
    if t_max == "auto":
        t_max = (world.width * world.height) // 2
    gen = ClauseGenerator(world, t_max)
    t_min = gen.solution_lower_bound
    if t_min > t_max:
        return None

    clauses = list[list[int]]()
    for t in range(t_min):
        clauses.extend(gen.generate(t))
        clauses.extend(gen.dependency_clauses(t))
    for t in range(t_min, t_max + 1):
        clauses.extend(gen.generate(t))
        clauses.extend(gen.dependency_clauses(t))
        # The mutual-exclusion clauses/assumptions are rebuilt per horizon and only fed to this
        # horizon's solver instance (they are not accumulated into `clauses`).
        mutual_clauses, assumptions = gen.forbid_mutual_cooperation()
        objective = gen.objective(t)
        model = solve_model(clauses + mutual_clauses + objective, assumptions=assumptions)
        if model is not None:
            return _to_plan(gen.decode_plan(model, t))
    return None


def requires_mutual_cooperation(world: World, t_max: int | Literal["auto"] = "auto") -> bool:
    """
    Return `True` if the world is solvable within `t_max` steps but **cannot** be solved without
    some pair of agents mutually cooperating (each helping the other across a laser beam).

    Returns `False` for worlds that are unsolvable within `t_max`, and for worlds that admit at
    least one mutual-cooperation-free solution.
    """
    if solve(world, t_max) is None:
        return False
    return solve_without_mutual_cooperation(world, t_max) is None


def solve_model(clauses: list[list[int]], *, assumptions: list[int] | None = None) -> list[int] | None:
    """
    Solve the SAT problem with the given clauses and assumptions, returning the literals' values
    if a solution is found, or `None` if the clauses are unsatisfyiable.
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
