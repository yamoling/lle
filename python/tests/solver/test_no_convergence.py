import pytest
from lle import Action, solve
from lle.characterization import profile_plan
from lle.solver import SolveMode, Solver

from ..world_layouts import (
    CONVERGENT_2_TIGHT,
    ConvergentCase,
    convergent_cases,
)


@pytest.mark.parametrize("test_case", convergent_cases(), ids=lambda case: case.id)
def test_no_convergence_mode_matches_world_specification(test_case: ConvergentCase):
    """The mode is unsatisfiable exactly when k-convergence is unavoidable."""
    plan = solve(
        test_case.layout.world(),
        test_case.t_max,
        mode=SolveMode.no_convergence(test_case.k),
    )
    assert (plan is None) == test_case.expected, f"Plan: {plan}"
    if plan is not None:
        assert not profile_plan(test_case.layout.world(), plan).is_convergent(test_case.k)


def test_canonical_convergent_layout_intended_witness():
    """Replay the intended five-step witness for the canonical convergence layout."""
    plan = [
        (Action.SOUTH, Action.STAY, Action.SOUTH),
        (Action.SOUTH, Action.NORTH, Action.SOUTH),
        (Action.STAY, Action.STAY, Action.SOUTH),
        (Action.STAY, Action.SOUTH, Action.SOUTH),
        (Action.STAY, Action.SOUTH, Action.SOUTH),
    ]
    world = CONVERGENT_2_TIGHT.world()
    profile = profile_plan(world, plan)
    assert profile.is_convergent(2)
    assert not profile.is_convergent(3)
    assert set(world.agents_positions) == set(world.exit_pos)


def test_no_convergence_cache_matches_fresh_solver_across_thresholds_and_horizons():
    """Reuse convergence clauses without leaking horizons or thresholds."""
    shared_solver = Solver(CONVERGENT_2_TIGHT.world(), 5)
    for mode, horizon, k in (
        (SolveMode.standard(), 5, None),
        (SolveMode.no_convergence(2), 5, 2),
        (SolveMode.no_convergence(3), 5, 3),
        (SolveMode.no_convergence(2), 4, 2),
        (SolveMode.no_convergence(3), 4, 3),
        (SolveMode.no_convergence(2), 5, 2),
    ):
        cached_plan = shared_solver.solve(mode, override_t_max=horizon)
        fresh_plan = Solver(CONVERGENT_2_TIGHT.world(), 5).solve(mode, override_t_max=horizon)
        assert (cached_plan is None) is (fresh_plan is None)
        if cached_plan is not None:
            assert fresh_plan is not None
            assert len(cached_plan) == len(fresh_plan)
        if k is not None:
            for plan in (cached_plan, fresh_plan):
                if plan is not None:
                    assert not profile_plan(CONVERGENT_2_TIGHT.world(), plan).is_convergent(k)
