import pytest
from lle.characterization.plan import profile_plan
from lle.solver import Solver

from ..world_layouts import (
    LEVEL_3,
    LEVEL_5,
    ScalarPropertyCase,
    scalar_cases_for,
)


@pytest.mark.parametrize("property_case", scalar_cases_for("asymmetric"), ids=lambda case: case.id)
def test_no_asymmetric_mode_matches_world_specification(property_case: ScalarPropertyCase):
    """The mode has a solution exactly when asymmetric help is avoidable.

    @ai-generated
    """
    solver = Solver(property_case.layout.world(), property_case.t_max)
    assert (solver.solve(mode="no-asymmetric") is None) is property_case.expected


def test_level_3_requires_agent_0_asymmetric_help():
    """Level 3's unavoidable asymmetric edge is from agent 0 to agent 1.

    @ai-generated
    """
    world = LEVEL_3.world()
    solver = Solver(world, 21)

    path = solver.solve()
    assert path is not None
    assert profile_plan(world, path).graph.asymmetric_edges() == {(0, 1)}
    assert solver.solve(mode="no-asymmetric") is None


def test_level_5_requires_agent_1_asymmetric_help_at_tmax_21():
    """At t=21, level 5 requires agent 1's asymmetric help edges.

    @ai-generated
    """
    world = LEVEL_5.world()
    solver = Solver(world, 21)

    path = solver.solve()
    assert path is not None
    assert profile_plan(world, path).graph.asymmetric_edges() == {
        (1, 0),
        (1, 2),
        (1, 3),
    }
    assert solver.solve(mode="no-asymmetric") is None


def test_level_5_has_longer_non_asymmetric_solution_at_tmax_25():
    """A longer horizon admits level 5's 24-step non-asymmetric plan."""
    world = LEVEL_5.world()
    solver = Solver(world, 25)

    path = solver.solve()
    non_asymmetric = solver.solve(mode="no-asymmetric")
    assert path is not None
    assert non_asymmetric is not None
    assert len(path) == 25
    assert len(non_asymmetric) == 25
