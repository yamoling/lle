from typing import Any, cast

import pytest

from lle import Action, World
from lle.characterization.plan import profile_plan
from lle.solver import Solver
from lle.solver.clauses import ClauseGenerator


def _assert_plan(world: World, t_max: int, mode: str = "standard"):
    """Return a solved plan and fail the test if the world is unexpectedly unsolved.

    @ai-generated
    """
    plan = Solver(world, t_max).solve(mode)
    assert plan is not None
    return plan


def _assignment_for_plan(world: World, plan):
    """Build a formula assignment pinned to `plan` with asymmetry support clauses.

    @ai-generated
    """
    gen = ClauseGenerator(world, len(plan))
    assignment = gen.assignment_for_trajectory(plan, len(plan))
    return gen, assignment


def test_literal_lookup_does_not_allocate():
    """`literal(...)` returns existing ids only; it must never create variables.

    @ai-generated
    """
    gen = ClauseGenerator(World.level(3), 21)

    before = gen.n_vars
    assert gen.literal("asymmetric", horizon=21) is None
    assert gen.n_vars == before

    _ = gen.characterization_clauses(21)
    after_support = gen.n_vars
    assert gen.literal("asymmetric", horizon=21) is not None
    assert gen.n_vars == after_support


def test_assignment_is_signed_literals_not_clauses():
    """Truth queries are evaluated in signed SAT assignments, not in clause lists.

    @ai-generated
    """
    world = World("S0 X")
    plan = _assert_plan(world, 2)
    gen, assignment = _assignment_for_plan(world, plan)

    assert isinstance(assignment, list)
    assert all(isinstance(literal, int) for literal in assignment)
    assert gen.value_in_assignment(assignment, "asymmetric", horizon=len(plan)) is False

    clauses = gen.characterization_clauses(len(plan))
    with pytest.raises(TypeError):
        gen.value_in_assignment(cast(Any, clauses), "asymmetric", horizon=len(plan))


def test_asymmetry_for_concrete_asymmetric_trajectory_matches_profile():
    """Formula-level asymmetry agrees with trajectory-level characterization.

    @ai-generated
    """
    world = World.level(3)
    plan = _assert_plan(world, 21)
    profile = profile_plan(world, plan)

    gen, assignment = _assignment_for_plan(world, plan)
    assert gen.value_in_assignment(assignment, "asymmetric", horizon=len(plan)) is profile.is_asymmetric is True
    assert gen.value_for_trajectory(plan, "asymmetric", horizon=len(plan)) is True


def test_asymmetry_false_for_concrete_independent_trajectory():
    """An independent feasible trajectory sets the materialized asymmetry variable false.

    @ai-generated
    """
    world = World("""
    S0 . S1
     . . .
     X . X""")
    plan = _assert_plan(world, 6)
    profile = profile_plan(world, plan)

    gen, assignment = _assignment_for_plan(world, plan)
    assert profile.is_asymmetric is False
    assert gen.literal("asymmetric", horizon=len(plan)) is not None
    assert gen.value_in_assignment(assignment, "asymmetric", horizon=len(plan)) is False
    assert gen.value_for_trajectory(plan, "asymmetric", horizon=len(plan)) is False


def test_non_asymmetric_cooperative_trajectory_level_5_long_horizon():
    """A known longer non-asymmetric level-5 trajectory is false in the formula too.

    @ai-generated
    """
    world = World.level(5)
    plan = _assert_plan(world, 25, mode="no-asymmetric")
    profile = profile_plan(world, plan)

    gen, assignment = _assignment_for_plan(world, plan)
    assert profile.is_asymmetric is False
    assert gen.value_in_assignment(assignment, "asymmetric", horizon=len(plan)) is False


def test_infeasible_trajectory_has_no_assignment():
    """Impossible trajectories fail before any derived truth value is reported.

    @ai-generated
    """
    world = World("S0 @ X")
    gen = ClauseGenerator(world, 1)

    with pytest.raises(ValueError):
        gen.assignment_for_trajectory([(Action.EAST,)], 1)

    with pytest.raises(ValueError):
        gen.value_for_trajectory([(Action.EAST,)], "asymmetric", horizon=1)


def test_true_help_edges_match_temporal_dependency_graph_edges():
    """True SAT `Help` variables match replay-detected dependency edges exactly.

    @ai-generated
    """
    world = World.level(3)
    plan = _assert_plan(world, 21)
    graph = profile_plan(world, plan).graph
    graph_edges = {(edge.helper, edge.beneficiary, edge.t) for edge in graph.edges}

    gen, assignment = _assignment_for_plan(world, plan)
    sat_edges = set(gen.true_help_edges_in_assignment(assignment, len(plan)))
    assert sat_edges == graph_edges
    assert set(gen.true_help_edges_for_trajectory(plan, len(plan))) == graph_edges


def test_agent_summary_variables_match_help_edge_aggregation():
    """`ProvidesHelp` and `IsHelped` agree with graph-level edge aggregation.

    @ai-generated
    """
    world = World.level(3)
    plan = _assert_plan(world, 21)
    graph = profile_plan(world, plan).graph
    horizon = len(plan)
    gen, assignment = _assignment_for_plan(world, plan)

    for agent in range(world.n_agents):
        expected_provides = any(edge.helper == agent for edge in graph.edges)
        expected_helped = any(edge.beneficiary == agent for edge in graph.edges)

        provides = gen.value_in_assignment(assignment, "provides_help", helper=agent, horizon=horizon)
        helped = gen.value_in_assignment(assignment, "is_helped", beneficiary=agent, horizon=horizon)

        if gen.literal("provides_help", helper=agent, horizon=horizon) is None:
            assert expected_provides is False
            assert provides is None
        else:
            assert provides is expected_provides

        if gen.literal("is_helped", beneficiary=agent, horizon=horizon) is None:
            assert expected_helped is False
            assert helped is None
        else:
            assert helped is expected_helped


def test_impossible_help_variable_is_none_not_false():
    """Unmaterialized impossible help variables remain `None` in lookup and assignment queries.

    @ai-generated
    """
    world = World("""
    S0 . S1
     . . .
     X . X""")
    plan = _assert_plan(world, 6)
    gen, assignment = _assignment_for_plan(world, plan)

    assert gen.literal("help", helper=0, beneficiary=1, t=0) is None
    assert gen.value_in_assignment(assignment, "help", helper=0, beneficiary=1, t=0) is None


def test_invalid_query_arguments_raise_value_error():
    """Invalid semantic variable queries fail with clear `ValueError`s.

    @ai-generated
    """
    gen = ClauseGenerator(World.level(3), 21)
    plan = _assert_plan(World.level(3), 21)

    with pytest.raises(ValueError):
        gen.literal("asymmetric")
    with pytest.raises(ValueError):
        gen.literal("help", helper=0, beneficiary=1)
    with pytest.raises(ValueError):
        gen.literal("unknown", horizon=0)
    with pytest.raises(ValueError):
        gen.value_for_trajectory(plan, "help", helper=0, beneficiary=1, horizon=len(plan))


def test_horizon_specific_asymmetry_variables_are_distinct():
    """Horizon-scoped derived variables do not alias across different horizons.

    @ai-generated
    """
    gen = ClauseGenerator(World.level(3), 7)
    _ = gen.characterization_clauses(6)
    _ = gen.characterization_clauses(7)

    lit_6 = gen.literal("asymmetric", horizon=6)
    lit_7 = gen.literal("asymmetric", horizon=7)
    assert lit_6 is not None
    assert lit_7 is not None
    assert lit_6 != lit_7

    with pytest.raises(ValueError):
        gen.assignment_for_trajectory([], 7)
