"""Incremental SAT solver that builds constraints incrementally for time-bounded solving."""

import random
from typing import Literal

from pysat.solvers import Minisat22  # pyright: ignore[reportMissingTypeStubs]

from ..world import Action, World
from .clauses import ClauseGenerator, SolveMode
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
        path_length: int | Literal["auto"] = "auto",
        *,
        mode: SolveModeLiteral | str | SolveMode = "standard",
        collect_gems: bool = False,
        shuffle: bool = False,
    ) -> list[tuple[Action, ...]] | None:
        """Find a plan with the requested length.

        `path_length` is the exact number of joint actions in the returned plan.
        When it is `"auto"` (the default), this solver's construction-time
        `t_max` is used. A requested length cannot exceed `t_max`.
        """
        if path_length == "auto":
            path_length = self.t_max
        elif path_length < 0:
            raise ValueError(f"path_length must be non-negative, got {path_length}.")
        elif path_length > self.t_max:
            raise ValueError(
                f"path_length={path_length} exceeds this solver's t_max={self.t_max}. Construct a new Solver with a larger t_max."
            )

        if path_length < self.solution_lower_bound:
            return None

        parsed_mode = _parse_mode(mode)
        clauses, assumptions = self.generator.generate(path_length, mode=parsed_mode, collect_gems=collect_gems)
        if shuffle:
            random.shuffle(clauses)
            random.shuffle(assumptions)
        model = solve_model(clauses, assumptions=assumptions)
        if model is None:
            return None
        return _to_plan(self.generator.decode_plan(model, path_length))

    def find_shortest(
        self,
        mode: SolveModeLiteral | str | SolveMode = "standard",
        *,
        t_min: int | None = None,
        collect_gems: bool = False,
        shuffle: bool = False,
    ) -> list[tuple[Action, ...]] | None:
        """Find the shortest plan from `t_min` through this solver's `t_max`.

        When `t_min` is omitted, the search begins at the heuristic
        `solution_lower_bound`. A supplied `t_min` cannot exceed `t_max`.
        Candidate lengths are checked in ascending order, one step at a
        time, so the first plan found is the shortest at or above the requested
        lower bound.
        """
        if t_min is None or t_min < self.solution_lower_bound:
            t_min = self.solution_lower_bound
        elif t_min > self.t_max:
            raise ValueError(f"t_min={t_min} exceeds this solver's t_max={self.t_max}.")

        for path_length in range(t_min, self.t_max + 1):
            plan = self.solve(
                path_length=path_length,
                mode=mode,
                collect_gems=collect_gems,
                shuffle=shuffle,
            )
            if plan is not None:
                return plan
        return None


def solve(
    world: World,
    t_max: int | Literal["auto"] = "auto",
    /,
    *,
    path_length: int | Literal["auto"] = "auto",
    mode: SolveModeLiteral | str | SolveMode = "standard",
    collect_gems: bool = False,
    shuffle: bool = False,
) -> list[tuple[Action, ...]] | None:
    """Solve `world` with a fixed solver horizon and requested path length.

    `t_max` configures the construction-time episode horizon. `path_length`
    selects the exact plan length for this query; when it is `"auto"` (the
    default), `t_max` is used.

    """
    resolved_t_max = _default_t_max(world) if t_max == "auto" else t_max
    return Solver(world, resolved_t_max).solve(
        path_length=path_length,
        mode=mode,
        collect_gems=collect_gems,
        shuffle=shuffle,
    )


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
