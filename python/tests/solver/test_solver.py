import importlib

import lle
import pytest
from lle import Action, World
from lle.solver import Solver

from ..world_layouts import (
    BLOCKED_UNSOLVABLE,
    LEVEL_6,
    ScalarPropertyCase,
    scalar_cases_for,
)


@pytest.mark.parametrize("property_case", scalar_cases_for("solvable"), ids=lambda case: case.id)
def test_standard_mode_matches_world_specification(property_case: ScalarPropertyCase):
    """Standard solving agrees with every declared solvability expectation."""
    plan = lle.solve(property_case.layout.world(), property_case.t_max)
    assert (plan is not None) is property_case.expected


def test_solve_simple_world_returns_shortest_plan():
    world = World("S0 . . X")
    plan = lle.solve(world, 15)
    assert plan is not None
    assert len(plan) == 3
    assert all(isinstance(row, tuple) for row in plan)
    assert all(isinstance(a, Action) for row in plan for a in row)


def test_solve_fixed_length():
    world = World("S0 . . X")
    plan = lle.solve(world, 5, 5)
    assert plan is not None
    assert len(plan) == 5


def test_solve_unsolvable_returns_none():
    # Agent walled off from the exit.
    assert lle.solve(BLOCKED_UNSOLVABLE.world(), 10) is None


def test_solve_default_t_max():
    # 2x2 grid: agent at (0,0), exit at (1,1). default t_max = (2*2)//2 = 2, which is sufficient.
    world = World("S0 .\n.  X")
    t_max = (world.width * world.height) // 2
    plan = lle.solve(world, "auto")  # default t_max
    assert plan is not None
    assert len(plan) == t_max


def test_solve_plan_is_executable():
    world = World("S0 . . X")
    plan = lle.solve(world, 4)
    assert plan is not None
    world.reset()
    for joint in plan:
        world.step(joint)
    assert all(a.has_arrived for a in world.agents)


def test_solve_path_is_executable_2agents():
    world = World("""
@  @ L1S @ X
S0 .  .  . .
S1 .  .  . .
@  @  .  @ X
""")
    plan = lle.solve(world, 10)
    assert plan is not None
    world.reset()
    for joint in plan:
        world.step(joint)
    assert all(a.has_arrived for a in world.agents)


def test_solve_level_6_world_is_executable():
    world = LEVEL_6.world()
    plan = lle.solve(world, 21)
    assert plan is not None
    world.reset()
    for joint in plan:
        world.step(joint)
    assert all(a.has_arrived for a in world.agents)


def test_not_solvable():
    """Not solvable because of the laser positioning: agent 1 cannot shield agent 0."""
    worlds = [
        """
 @ L1S  .  @
S0  .   .  X
S1  .   .  X
 @  .  L1N @"""
    ]
    for ws in worlds:
        world = World(ws)
        assert lle.solve(world, 10) is None


def test_two_same_colour_lasers_blocking_distinct_routes_is_unsat():
    # Agent 1 (colour 1) can only leave through (0, 4).
    # Agent 0 is sealed into column 1 and cannot block either beam, so the level is genuinely unsolvable.
    # Modelling only one of the two beams would (wrongly) leave a route open.
    world = World("""
L0S @  L0S @ S0
 X  S1  X  @ X
""")
    assert lle.solve(world, 6) is None


def test_two_same_colour_same_direction_lasers_with_clear_lanes_is_solvable():
    world = World("""
S1 L0S L0S S0
.  .   .   .
X  X   .   .
""")
    assert lle.solve(world, 4) is not None


def test_two_same_colour_crossing_lasers_keep_independent_beams():
    # A colour may own many lasers, in several directions, whose beams CROSS. Every laser
    # here is colour 0: three south lasers (columns 1, 2, 3), one east laser (row 1) and one
    # north laser (column 4). The east beam along row 1 crosses all four vertical beams. Each
    # beam must stay independent at every crossing rather than being forced to coincide:
    # a crossing cell carries one beam variable per (direction, source), not a single shared one.
    world = World("""
.   L0S L0S L0S X
L0E .   .   .   .
S0  .   .   .   .
S1  .   .   .   L0N
.   .   .   .   X
""")
    assert lle.solve(world, 6) is not None


def test_solver_override_t_max_cannot_exceed_construction_bound():
    solver = Solver(World("S0 . . X"), 5)
    with pytest.raises(ValueError, match="exceeds this solver's t_max"):
        solver.solve(override_t_max=6)


def test_collect_gems_is_per_solve_call():
    world = World("S0 G X")
    solver = Solver(world, 2)
    assert solver.solve(collect_gems=False) is not None
    assert solver.solve(collect_gems=True) is not None


def test_solver_decodes_only_the_shortest_satisfiable_model(monkeypatch: pytest.MonkeyPatch):
    """Binary search decodes once after retaining its shortest satisfiable model.

    @ai-generated
    """

    class FakeGenerator:
        """Minimal generator test double that records model decoding."""

        solution_lower_bound = 0

        def __init__(self) -> None:
            self.decode_calls: list[int] = []

        def generate(self, horizon: int, *, mode: object, collect_gems: bool):
            """Encode the queried horizon in a single test clause.

            @ai-generated
            """
            return [[horizon]], []

        def decode_plan(self, model: list[int], horizon: int):
            """Record the selected horizon and return a plan of matching length.

            @ai-generated
            """
            self.decode_calls.append(horizon)
            return [[Action.STAY] for _ in range(horizon)]

    def fake_solve_model(clauses: list[list[int]], *, assumptions: list[int] | None = None):
        """Return SAT for horizons at least three.

        @ai-generated
        """
        return [1] if clauses[0][0] >= 3 else None

    solver_module = importlib.import_module("lle.solver.solver")
    monkeypatch.setattr(solver_module, "solve_model", fake_solve_model)

    solver = Solver.__new__(Solver)
    solver.world = World("S0 X")
    solver.t_max = 10
    generator = FakeGenerator()
    solver.generator = generator  # type: ignore[assignment]

    plan = solver.solve()

    assert plan is not None
    assert len(plan) == 3
    assert generator.decode_calls == [3]
