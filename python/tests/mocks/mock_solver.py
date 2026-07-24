from __future__ import annotations

from collections.abc import Mapping
from typing import Literal

from lle.solver import Solver
from lle.solver.clauses import SolveMode
from lle.solver.types import SolveModeLiteral
from lle.world import Action, World
from typing_extensions import override


class MockSolver(Solver):
    """Test double for `Solver` that returns canned results and records calls."""

    def __init__(
        self,
        world: World,
        t_max: int | Literal["auto"] = "auto",
        *,
        responses: Mapping[str, list[tuple[Action, ...]] | None] | None = None,
    ) -> None:
        super().__init__(world, t_max)
        self._responses: dict[str, list[tuple[Action, ...]] | None] = dict(responses or {})
        self._calls: list[str] = []

    @property
    @override
    def solution_lower_bound(self) -> int:
        return 0

    @override
    def solve(
        self,
        mode: SolveModeLiteral | str | SolveMode = "standard",
        *,
        t_min: int = 0,
        override_t_max: int | None = None,
        collect_gems: bool = False,
        shuffle: bool = False,
    ) -> list[tuple[Action, ...]] | None:
        """Return the configured result for `mode` and record the call."""
        mode_str = str(mode)
        self._calls.append(mode_str)
        if mode_str not in self._responses:
            raise AssertionError(f"Unexpected solver mode: {mode_str}")
        return self._responses[mode_str]
