from __future__ import annotations

from collections.abc import Mapping

from lle.solver import Solver
from lle.solver.constraints import SolveMode
from lle.solver.types import SolveModeLiteral
from lle.world import Action, World


class MockSolver(Solver):
    """Test double for `Solver` that returns canned results and records calls."""

    def __init__(
        self,
        world: World,
        t_max: int | str = "auto",
        *,
        responses: Mapping[str, list[tuple[Action, ...]] | None] | None = None,
    ) -> None:
        self.world = world
        self.t_max: int = (world.width * world.height) // 2 if t_max == "auto" else int(t_max)
        self.responses = dict(responses or {})
        self.calls: list[str] = []

    @property
    def solution_lower_bound(self) -> int:
        return 0

    def solve(
        self,
        mode: SolveModeLiteral | str | SolveMode = "standard",
        *,
        t_min: int = 0,
        override_t_max: int | None = None,
        collect_gems: bool = False,
    ) -> list[tuple[Action, ...]] | None:
        """Return the configured result for `mode` and record the call."""
        mode_str = str(mode)
        self.calls.append(mode_str)
        if mode_str not in self.responses:
            raise AssertionError(f"Unexpected solver mode: {mode_str}")
        return self.responses[mode_str]
