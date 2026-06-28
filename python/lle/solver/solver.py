"""Incremental SAT solver that builds constraints incrementally for time-bounded solving."""

from typing import Literal, overload

from pysat.solvers import Minisat22  # pyright: ignore[reportMissingTypeStubs]

from ..world import Action, World
from .constraints import ClauseGenerator, SolveMode
from .types import SolveModeLiteral


def _default_t_max(world: World) -> int:
    return (world.width * world.height) // 2


def _parse_mode(mode: SolveModeLiteral | str | SolveMode) -> SolveMode:
    if isinstance(mode, str):
        return SolveMode.from_str(mode)
    return mode


class Solver:
    """Reusable SAT solver facade for one world and maximum horizon.

    A `Solver` owns one Rust `ClauseGenerator`, so repeated calls with different solve modes reuse
    the same cached domain clauses.
    """

    world: World
    t_max: int
    generator: ClauseGenerator

    def __init__(self, world: World, t_max: int | Literal["auto"] = "auto") -> None:
        self.world = world
        self.t_max = _default_t_max(world) if t_max == "auto" else t_max
        self.generator = ClauseGenerator(world, self.t_max)

    @property
    def solution_lower_bound(self) -> int:
        return self.generator.solution_lower_bound

    def solve(
        self,
        mode: SolveModeLiteral | str | SolveMode = "standard",
        *,
        t_min: int = 0,
        override_t_max: int | None = None,
        collect_gems: bool = False,
    ) -> list[tuple[Action, ...]] | None:
        """Find the shortest plan for this solver's world.

        `override_t_max` may restrict the horizon for this call, but it cannot exceed the solver's
        construction-time `t_max` because the underlying Rust generator was built for that bound.
        """
        if override_t_max is None:
            t_max = self.t_max
        elif override_t_max > self.t_max:
            raise ValueError(
                f"override_t_max={override_t_max} exceeds this solver's t_max={self.t_max}. Construct a new Solver with a larger t_max."
            )
        else:
            t_max = override_t_max

        parsed_mode = _parse_mode(mode)
        t_min = max(self.solution_lower_bound, t_min)
        if t_min > t_max:
            return None

        low = t_min
        high = t_max
        best_plan = None
        while low <= high:
            mid = (low + high) // 2
            clauses, assumptions = self.generator.generate(mid, mode=parsed_mode, collect_gems=collect_gems)
            model = solve_model(clauses, assumptions=assumptions)
            if model is not None:
                best_plan = _to_plan(self.generator.decode_plan(model, mid))
                high = mid - 1
            else:
                low = mid + 1

        return best_plan


@overload
def solve(
    world: World,
    t_max: int | Literal["auto"] = "auto",
    /,
    *,
    mode: SolveModeLiteral | str | SolveMode = "standard",
    collect_gems: bool = False,
) -> list[tuple[Action, ...]] | None: ...


@overload
def solve(
    world: World,
    t_min: int,
    t_max: int | Literal["auto"] = "auto",
    /,
    *,
    mode: SolveModeLiteral | str | SolveMode = "standard",
    collect_gems: bool = False,
) -> list[tuple[Action, ...]] | None: ...


def solve(
    world: World,
    /,
    *min_max: int | Literal["auto"],
    mode: SolveModeLiteral | str | SolveMode = "standard",
    collect_gems: bool = False,
):
    """
    Find the shortest plan within the time range [t_min, t_max] (both ends included).

    # Arguments:
        - `t_min`: The minimum time step to consider.
        - `t_max`: The maximum time step to consider. Defaults to (width * height) // 2.
        - `mode`: The solving mode. Check the `SolveMode` enum for more information.
        - `collect_gems`: Whether all gems must be collected before solving succeeds.
    """
    match min_max:
        case ():
            return Solver(world).solve(mode, collect_gems=collect_gems)
        case (t_max,):
            return Solver(world, t_max).solve(mode, collect_gems=collect_gems)
        case (int(t_min), t_max):
            return Solver(world, t_max).solve(mode, t_min=t_min, collect_gems=collect_gems)
        case _:
            raise ValueError(f"Invalid arguments: (world, {min_max})")


def solve_model(clauses: list[list[int]], *, assumptions: list[int] | None = None) -> list[int] | None:
    """
    Solve the SAT problem with the given clauses and assumptions, returning the literals' values
    if a solution is found, or `None` if the clauses are unsatisfiable.
    """
    if assumptions is None:
        assumptions = []
    with Minisat22(bootstrap_with=clauses) as sat_solver:
        res = bool(sat_solver.solve(assumptions=assumptions))  # pyright: ignore[reportUnknownArgumentType, reportUnknownMemberType]
        if res:
            model: list[int] | None = sat_solver.get_model()  # pyright: ignore[reportUnknownVariableType]
            assert model is not None
            return model  # pyright: ignore[reportUnknownVariableType]


def _to_plan(joint_actions: list[list[Action]]) -> list[tuple[Action, ...]]:
    return [tuple(joint) for joint in joint_actions]
