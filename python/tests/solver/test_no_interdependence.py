import pytest
from lle import solve
from lle.solver import SolveMode, Solver

from ..world_layouts import LEVEL_6, InterdependentCase, interdependent_cases


@pytest.mark.parametrize("test_case", interdependent_cases(), ids=lambda case: case.id)
def test_no_interdependence_matches_specification(test_case: InterdependentCase):
    """The mode is unsatisfiable exactly when the cycle order is unavoidable."""
    plan = solve(
        test_case.layout.world(),
        test_case.t_max,
        mode=f"no-interdependence-{test_case.order}",
    )
    assert (plan is None) is test_case.expected


def test_interdependence_cache_matches_fresh_solver_across_probe_order():
    """Descending horizons and exact orders do not leak cached temporal clauses.

    @ai-generated
    """
    shared = Solver(LEVEL_6.world(), 21)
    probes = [
        (SolveMode.no_interdependence(3), 21),
        (SolveMode.no_interdependence(2), 21),
        (SolveMode.no_interdependence(3), 18),
        (SolveMode.no_interdependence(2), 20),
        (SolveMode.no_interdependence(3), 21),
    ]

    for mode, horizon in probes:
        cached = shared.solve(mode, override_t_max=horizon)
        fresh = Solver(LEVEL_6.world(), 21).solve(mode, override_t_max=horizon)
        assert (cached is None) is (fresh is None)
        if cached is not None:
            assert fresh is not None
            assert len(cached) == len(fresh)
