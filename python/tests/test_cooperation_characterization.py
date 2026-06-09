"""Tests for ``lle.cooperation.characterize`` and ``WorldCharacterization``.

Covers:
- the proof-of-concept level: agent 1 depends on agent 0 only for short plans;
- query-surface semantics (``depends`` / ``is_independent`` / ``requires_mutual`` /
  ``requires_chain``) including edge cases;
- sanity checks on levels 4 and 6 (mutual cooperation thresholds);
- internal consistency of the global ``fully_independent_threshold`` against the
  concrete ``solve`` / ``solve_no_cooperation`` solvers;
- degenerate worlds (single agent, no lasers).

Solver-backed tests use the project's 60-second timeout convention (CLAUDE.md).
"""

from __future__ import annotations

import pytest
from lle import World, characterize
from lle.cooperation import WorldCharacterization
from lle.cooperation import characterize as characterize_from_subpackage
from lle.solver.solver import solve, solve_no_cooperation

# The proof-of-concept level. Agent 0's laser fires east along row 1; agent 1
# (a different colour) must either be let through by agent 0 (short plans) or
# detour around the wall at (1, 4) (longer plans).
POC_LEVEL = """
 .   .  S0  S1  .   .
L0E  .   .   .  @   .
 .   .   .   .  .   .
 .   .   .   .  .   .
 X   X   .   .  .   .
"""


@pytest.fixture(scope="module")
def poc_props() -> WorldCharacterization:
    return characterize(World(POC_LEVEL), t_max=12)


def test_reexported_consistently():
    """``characterize`` is reachable from both the top level and the subpackage."""
    assert characterize is characterize_from_subpackage


def test_poc_dependency_required_for_short_plans(poc_props: WorldCharacterization):
    # Agent 1 needs agent 0's help for every plan up to length 9.
    assert poc_props.depends(1, 0, 9) is True
    # Once there is enough time budget to detour, the dependency is gone.
    assert poc_props.depends(1, 0, 10) is False
    assert poc_props.depends(1, 0, 12) is False


def test_poc_laser_colour_never_depends_on_other(poc_props: WorldCharacterization):
    # Agent 0 owns the laser, so it never needs agent 1's help.
    assert poc_props.first_solvable_length is not None
    for t in range(poc_props.first_solvable_length, poc_props.t_max + 1):
        assert poc_props.depends(0, 1, t) is False


def test_poc_independence_threshold(poc_props: WorldCharacterization):
    assert poc_props.independence_threshold[(1, 0)] == 10
    # Agent 0 is trivially independent from the first solvable length.
    assert poc_props.independence_threshold[(0, 1)] == poc_props.first_solvable_length


def test_poc_is_independent_transition(poc_props: WorldCharacterization):
    assert poc_props.is_independent(9) is False
    assert poc_props.is_independent(10) is True
    assert poc_props.fully_independent_threshold == 10


def test_poc_first_solvable_length(poc_props: WorldCharacterization):
    world = World(POC_LEVEL)
    shortest = solve(world, 12)
    assert shortest is not None
    assert poc_props.first_solvable_length == len(shortest)
    assert poc_props.solution_lower_bound <= len(shortest)


def test_full_independence_agrees_with_no_blocking(poc_props: WorldCharacterization):
    """The global independence threshold matches the shortest no-cooperation plan.

    Under correct strict-laser semantics the two notions coincide: in any valid
    plan, a different-colour agent can only stand on a beam tile if the source
    agent blocks it upstream (a help event), so "no other agent ever on a beam"
    (``solve_no_cooperation``) is equivalent to "no help event ever"
    (``fully_independent``). Hence ``fully_independent_threshold == len(plan)``.
    """
    no_coop = solve_no_cooperation(World(POC_LEVEL), t_max=12)
    assert no_coop is not None
    assert poc_props.fully_independent_threshold == len(no_coop) == 10


def test_depends_is_false_before_first_solvable(poc_props: WorldCharacterization):
    # No plan exists shorter than the lower bound, so nothing can be "required".
    assert poc_props.first_solvable_length is not None
    assert poc_props.depends(1, 0, poc_props.first_solvable_length - 1) is False


def test_single_agent_has_no_pairs():
    props = characterize(World("S0 . . X"), t_max=6)
    assert props.independence_threshold == {}
    # Vacuously false: there are no ordered pairs to depend on.
    assert props.depends(0, 0, 5) is False


def test_world_without_lasers_is_fully_independent_immediately():
    # Two agents on independent rows, no lasers: no cooperation is ever possible.
    props = characterize(World("S0 . X\nS1 . X"), t_max=6)
    assert props.first_solvable_length is not None
    # Every pair is trivially independent at the first solvable length.
    for threshold in props.independence_threshold.values():
        assert threshold == props.first_solvable_length
    assert props.fully_independent_threshold == props.first_solvable_length
    assert props.is_independent(props.first_solvable_length) is True
    assert props.depends(1, 0, props.t_max) is False


def test_unsolvable_within_horizon_reports_no_plan():
    # t_max below the solution lower bound: nothing is solvable.
    world = World(POC_LEVEL)
    props = characterize(world, t_max=2)
    assert props.solution_lower_bound > props.t_max
    assert props.first_solvable_length is None
    assert props.fully_independent_threshold is None
    assert props.depends(1, 0, 2) is False
    assert props.is_independent(2) is False


# ---------------------------------------------------------------------------
# Mutual cooperation
# ---------------------------------------------------------------------------


def test_poc_no_mutual_required():
    # The PoC level is one-directional: agent 0 helps agent 1, but not vice-versa.
    # So no mutual pair can form → requires_mutual is always False.
    props = characterize(World(POC_LEVEL), t_max=12)
    assert props.mutual_free_threshold == props.first_solvable_length
    for t in range(props.first_solvable_length or 0, props.t_max + 1):
        assert props.requires_mutual(t) is False


def test_level4_requires_mutual_from_first_solvable():
    # Level 4: every plan of length >= 10 requires mutual cooperation (both agents
    # help each other), and the level is unsolvable for t < 10.
    props = characterize(World.level(4), t_max=15)
    assert props.first_solvable_length == 10
    assert props.requires_mutual(9) is False   # vacuous: no plan exists yet
    assert props.requires_mutual(10) is True
    assert props.requires_mutual(15) is True
    assert props.mutual_free_threshold is None  # no mutual-free plan exists <= t_max


def test_level6_requires_mutual_from_first_solvable():
    # Level 6: same property with a higher horizon.
    props = characterize(World.level(6), t_max=25)
    assert props.first_solvable_length == 21
    assert props.requires_mutual(20) is False
    assert props.requires_mutual(21) is True
    assert props.mutual_free_threshold is None


def test_requires_mutual_false_before_first_solvable():
    props = characterize(World.level(4), t_max=15)
    assert props.first_solvable_length is not None
    assert props.requires_mutual(props.first_solvable_length - 1) is False


def test_no_laser_world_never_requires_mutual():
    # Two independent agents with no lasers: cooperation is impossible.
    props = characterize(World("S0 . X\nS1 . X"), t_max=6)
    assert props.first_solvable_length is not None
    assert props.mutual_free_threshold == props.first_solvable_length
    for t in range(props.first_solvable_length, props.t_max + 1):
        assert props.requires_mutual(t) is False


# ---------------------------------------------------------------------------
# Temporal chain
# ---------------------------------------------------------------------------


def test_two_agent_world_never_requires_chain():
    # With only 2 agents there are no distinct triples (a, b, c), so no chain
    # can ever form. chain_free_threshold equals first_solvable_length.
    props = characterize(World.level(4), t_max=15)
    assert props.first_solvable_length is not None
    assert props.chain_free_threshold == props.first_solvable_length
    for t in range(props.first_solvable_length, props.t_max + 1):
        assert props.requires_chain(t) is False


def test_requires_chain_false_before_first_solvable():
    props = characterize(World.level(6), t_max=25)
    assert props.first_solvable_length is not None
    assert props.requires_chain(props.first_solvable_length - 1) is False


def test_chain_semantics_independent_of_mutual():
    # Mutual cooperation (A↔B) is not a chain (it requires a third agent C).
    # Level 4 has 2 agents so mutual can hold without any chain.
    props = characterize(World.level(4), t_max=15)
    assert props.requires_mutual(10) is True
    assert props.requires_chain(15) is False  # 2 agents → chain impossible
