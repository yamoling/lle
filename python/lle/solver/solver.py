"""Incremental SAT solver that builds constraints incrementally for time-bounded solving.

Clause generation and SAT solving (CaDiCaL) both happen in Rust: the clauses never cross into Python.
"""

from typing import Literal

from ..world import Action, World
from .clauses import SatSolver, SolveMode
from .clauses import Solver as _RustSolver
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

    def __init__(self, world: World, t_max: int | Literal["auto"] = "auto") -> None:
        self.world = world
        self.t_max = _default_t_max(world) if t_max == "auto" else t_max
        self._solver = _RustSolver(world, self.t_max)

    @property
    def solution_lower_bound(self) -> int:
        return self._solver.solution_lower_bound

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

        The returned plan completes exactly at its last step: agents never all stand on an exit
        earlier, so a plan of length `n` is never a shorter plan padded with `Stay`s. A world may
        therefore be solvable at one length and not at a longer one.
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

        plan = self._solver.solve(path_length, mode=_parse_mode(mode), collect_gems=collect_gems, shuffle=shuffle)
        return _to_plan(plan)

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

        The whole search retains one SAT solver instance and one incremental clause stream fixed
        to `mode` and `collect_gems`, so later, larger horizons only add the clauses newly needed
        instead of resending everything checked so far. This solver's other methods (`solve`, and
        any later call to `find_shortest` with a different mode) are unaffected: each call to
        `find_shortest` starts its own stream from scratch.
        """
        if t_min is None or t_min < self.solution_lower_bound:
            t_min = self.solution_lower_bound
        elif t_min > self.t_max:
            raise ValueError(f"t_min={t_min} exceeds this solver's t_max={self.t_max}.")

        plan = self._solver.find_shortest(mode=_parse_mode(mode), t_min=t_min, collect_gems=collect_gems, shuffle=shuffle)
        return _to_plan(plan)


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
    sat_solver = SatSolver()
    sat_solver.append_formula(clauses)
    if sat_solver.solve(assumptions or []):
        return sat_solver.get_model()
    return None


def _to_plan(joint_actions: list[list[Action]] | None) -> list[tuple[Action, ...]] | None:
    if joint_actions is None:
        return None
    return [tuple(joint) for joint in joint_actions]
