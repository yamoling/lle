import pytest
from lle import solve
from lle.solver import Solver

from ..world_layouts import ONE_WAY_DETOUR, ScalarPropertyCase, scalar_cases_for


@pytest.mark.parametrize("test_case", scalar_cases_for("independent"), ids=lambda case: case.id)
def test_no_cooperation_mode_matches_world_specification(test_case: ScalarPropertyCase):
    """The mode has a solution exactly when independent solving is possible."""
    plan = solve(
        test_case.layout.world(),
        test_case.t_max,
        mode="no-cooperation",
    )
    assert (plan is not None) == test_case.expected


def test_reusable_solver_across_modes():
    """One solver can find both the short cooperative and longer independent plans."""
    solver = Solver(ONE_WAY_DETOUR.world(), 10)
    standard = solver.solve("standard")
    no_cooperation = solver.solve("no-cooperation")
    assert standard is not None
    assert no_cooperation is not None
    assert len(standard) < len(no_cooperation)


def test_independent_detour_becomes_available_at_t10():
    """The no-cooperation mode crosses its satisfiability threshold at t=10."""
    world = ONE_WAY_DETOUR.world()
    assert solve(world, 9, mode="no-cooperation") is None
    assert solve(world, 10, mode="no-cooperation") is not None
