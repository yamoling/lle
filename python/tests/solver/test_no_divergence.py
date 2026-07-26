import pytest
from lle import Action, solve
from lle.characterization import profile_plan
from lle.solver import SolveMode, Solver

from ..world_layouts import (
    DIVERGENT_2_TIGHT,
    DIVERGENT_2_WITH_DETOUR,
    LEVEL_6,
    DivergentCase,
    divergent_cases,
)

DIRECT_CROSSING = [
    (Action.NORTH, Action.NORTH, Action.NORTH),
    (Action.NORTH, Action.NORTH, Action.NORTH),
]
"""The two-step divergent witness shared by both canonical divergence layouts."""

DETOUR_PLAN = [
    (Action.NORTH, Action.NORTH, Action.SOUTH),
    (Action.NORTH, Action.NORTH, Action.EAST),
    (Action.STAY, Action.STAY, Action.EAST),
    (Action.STAY, Action.STAY, Action.NORTH),
    (Action.STAY, Action.STAY, Action.NORTH),
    (Action.STAY, Action.STAY, Action.NORTH),
]
"""The six-step plan in which agent 2 walks around the wall instead of being helped."""


@pytest.mark.parametrize("test_case", divergent_cases(), ids=lambda case: case.id)
def test_no_divergence_mode_matches_world_specification(test_case: DivergentCase):
    """The mode is unsatisfiable exactly when k-divergence is unavoidable.

    @ai-generated
    """
    plan = solve(
        test_case.layout.world(),
        test_case.t_max,
        mode=SolveMode.no_divergence(test_case.k),
    )
    assert (plan is None) == test_case.expected, f"Plan: {plan}"
    if plan is not None:
        assert not profile_plan(test_case.layout.world(), plan).is_divergent(test_case.k)


def test_canonical_divergent_layout_intended_witness():
    """Replay the intended two-step witness for the canonical divergence layout.

    @ai-generated
    """
    world = DIVERGENT_2_TIGHT.world()
    profile = profile_plan(world, DIRECT_CROSSING)
    assert profile.graph.flattened_edges() == {(0, 1), (0, 2)}
    assert profile.is_divergent(2)
    assert not profile.is_divergent(3)
    assert not profile.is_convergent(2)
    assert set(world.agents_positions) == set(world.exit_pos)


def test_detour_layout_intended_avoiding_witness():
    """The explicit detour plan is a winning, non-divergent trajectory.

    @ai-generated
    """
    world = DIVERGENT_2_WITH_DETOUR.world()
    profile = profile_plan(world, DETOUR_PLAN)
    assert profile.graph.flattened_edges() == {(0, 1)}
    assert not profile.is_divergent(2)
    assert set(world.agents_positions) <= set(world.exit_pos)


def test_detour_layout_short_horizon_witness_is_divergent():
    """Below the detour horizon, the only winning shape is the divergent crossing.

    @ai-generated
    """
    world = DIVERGENT_2_WITH_DETOUR.world()
    profile = profile_plan(world, DIRECT_CROSSING)
    assert profile.is_divergent(2)
    assert set(world.agents_positions) <= set(world.exit_pos)


@pytest.mark.parametrize("t_max, expected", [(2, True), (5, True), (6, False)])
def test_required_divergence_changes_at_the_detour_horizon(t_max: int, expected: bool):
    """Required divergence is horizon-dependent and flips once the detour fits.

    @ai-generated
    """
    plan = solve(DIVERGENT_2_WITH_DETOUR.world(), t_max, mode=SolveMode.no_divergence(2))
    assert (plan is None) == expected
    if plan is not None:
        assert not profile_plan(DIVERGENT_2_WITH_DETOUR.world(), plan).is_divergent(2)


def test_threshold_at_the_agent_count_behaves_like_standard_solving():
    """A structurally unreachable threshold adds no blocker, so the plan length is unchanged.

    @ai-generated
    """
    world = LEVEL_6.world
    n_agents = world().n_agents
    standard = solve(world(), 21)
    boundary = solve(world(), 21, mode=SolveMode.no_divergence(n_agents))
    assert standard is not None
    assert boundary is not None
    assert len(boundary) == len(standard)


def test_no_divergence_cache_matches_fresh_solver_across_modes_and_horizons():
    """Reuse divergence clauses without leaking horizons, thresholds, or orientations.

    @ai-generated
    """
    shared_solver = Solver(DIVERGENT_2_WITH_DETOUR.world(), 8)
    for mode, horizon, k in (
        (SolveMode.standard(), 8, None),
        (SolveMode.no_divergence(2), 8, 2),
        (SolveMode.no_convergence(2), 8, None),
        (SolveMode.no_divergence(3), 5, 3),
        (SolveMode.no_divergence(2), 5, 2),
        (SolveMode.no_divergence(2), 8, 2),
    ):
        cached_plan = shared_solver.solve(mode, override_t_max=horizon)
        fresh_plan = Solver(DIVERGENT_2_WITH_DETOUR.world(), 8).solve(mode, override_t_max=horizon)
        assert (cached_plan is None) is (fresh_plan is None), f"{mode} at {horizon}"
        if cached_plan is not None:
            assert fresh_plan is not None
            assert len(cached_plan) == len(fresh_plan)
        if k is not None:
            for plan in (cached_plan, fresh_plan):
                if plan is not None:
                    assert not profile_plan(DIVERGENT_2_WITH_DETOUR.world(), plan).is_divergent(k)
