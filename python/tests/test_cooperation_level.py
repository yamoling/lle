"""Tests for lle.cooperation_level and the CooperationLevel enum."""

from __future__ import annotations

from typing import get_args

import lle
import pytest
from lle import CooperationLevel, World
from lle.solver import CooperationLevelStr

# ---------------------------------------------------------------------------
# Enum invariants
# ---------------------------------------------------------------------------


def test_cooperation_level_is_str_compatible():
    assert CooperationLevel.COOPERATIVE == "cooperative"
    assert CooperationLevel.FULLY_COUPLED == "fully-coupled"
    assert CooperationLevel.ASYMMETRIC == "asymmetric"


def test_cooperation_level_round_trips_through_string():
    for level in CooperationLevel:
        assert CooperationLevel(level.value) is level


def test_cooperative_subtypes_excludes_unsolvable_and_independent():
    subtypes = CooperationLevel.cooperative_subtypes()
    assert CooperationLevel.INDEPENDENT not in subtypes


def test_is_at_least_reflexive():
    for level in CooperationLevel:
        assert level.is_at_least(level)


def test_is_at_least_matches_definition_order_for_all_pairs():
    levels = list(CooperationLevel)
    for i, left in enumerate(levels):
        for j, right in enumerate(levels):
            assert left.is_at_least(right) == (i >= j)
            # Also accept the string form of the other level.
            assert left.is_at_least(right.value) == (i >= j)


def test_typing_cooperation_level_str():
    # Every variant of CooperationLevelStr matches one CooperationLevel
    for s in get_args(CooperationLevelStr):
        assert any(s == c for c in CooperationLevel)
    # Every CooperationLevel is a valid CooperationLevelStr
    for c in CooperationLevel:
        assert c in get_args(CooperationLevelStr)


def test_is_at_least_cooperative():
    for other in CooperationLevel:
        if other.is_cooperative:
            assert other.is_at_least(CooperationLevel.COOPERATIVE)


@pytest.mark.parametrize(
    "tested, expected_false, expected_true",
    [
        (
            CooperationLevel.INDEPENDENT,
            [
                CooperationLevel.COOPERATIVE,
                CooperationLevel.ASYMMETRIC,
                CooperationLevel.CHAIN,
                CooperationLevel.DISTRIBUTED,
                CooperationLevel.MUTUAL,
                CooperationLevel.FULLY_COUPLED,
            ],
            [],
        ),
        (
            CooperationLevel.COOPERATIVE,
            [
                CooperationLevel.ASYMMETRIC,
                CooperationLevel.CHAIN,
                CooperationLevel.DISTRIBUTED,
                CooperationLevel.MUTUAL,
                CooperationLevel.FULLY_COUPLED,
            ],
            [CooperationLevel.INDEPENDENT],
        ),
        (
            CooperationLevel.ASYMMETRIC,
            [CooperationLevel.CHAIN, CooperationLevel.DISTRIBUTED, CooperationLevel.MUTUAL, CooperationLevel.FULLY_COUPLED],
            [CooperationLevel.INDEPENDENT, CooperationLevel.COOPERATIVE],
        ),
        (
            CooperationLevel.CHAIN,
            [CooperationLevel.DISTRIBUTED, CooperationLevel.MUTUAL, CooperationLevel.FULLY_COUPLED],
            [CooperationLevel.INDEPENDENT, CooperationLevel.COOPERATIVE, CooperationLevel.ASYMMETRIC],
        ),
        (
            CooperationLevel.DISTRIBUTED,
            [CooperationLevel.MUTUAL, CooperationLevel.FULLY_COUPLED],
            [CooperationLevel.INDEPENDENT, CooperationLevel.COOPERATIVE, CooperationLevel.ASYMMETRIC, CooperationLevel.CHAIN],
        ),
        (
            CooperationLevel.MUTUAL,
            [CooperationLevel.FULLY_COUPLED],
            [
                CooperationLevel.INDEPENDENT,
                CooperationLevel.COOPERATIVE,
                CooperationLevel.ASYMMETRIC,
                CooperationLevel.CHAIN,
                CooperationLevel.DISTRIBUTED,
            ],
        ),
        (
            CooperationLevel.FULLY_COUPLED,
            [],
            [
                CooperationLevel.INDEPENDENT,
                CooperationLevel.COOPERATIVE,
                CooperationLevel.ASYMMETRIC,
                CooperationLevel.CHAIN,
                CooperationLevel.DISTRIBUTED,
                CooperationLevel.MUTUAL,
            ],
        ),
    ],
)
def test_is_at_least(tested: CooperationLevel, expected_false: list[CooperationLevel], expected_true: list[CooperationLevel]):
    for other in expected_false:
        assert not tested.is_at_least(other), f"{tested} should not be at least {other}"
    for other in expected_true:
        assert tested.is_at_least(other), f"{tested} should be at least {other}"


def test_is_at_least_rejects_unknown_string():
    with pytest.raises(ValueError):
        CooperationLevel.INDEPENDENT.is_at_least("not-a-real-level")  # type: ignore


# ---------------------------------------------------------------------------
# Public function against canonical LLE levels
# ---------------------------------------------------------------------------


def test_cooperation_level_returns_enum_member():
    world = World.level(1)
    assert isinstance(lle.cooperation_level(world), CooperationLevel)


def test_cooperation_level_independent_for_trivial_world():
    assert lle.cooperation_level(World("S0 . X"), t_max=5) is CooperationLevel.INDEPENDENT


def test_cooperation_level_classifies_level_3_as_asymmetric():
    assert lle.cooperation_level(World.level(3), t_max=10) is CooperationLevel.ASYMMETRIC


def test_cooperation_level_classifies_level_4_as_fully_coupled():
    assert lle.cooperation_level(World.level(4), t_max=10) is CooperationLevel.FULLY_COUPLED


def test_cooperation_level_classifies_level_5_as_asymmetric():
    assert lle.cooperation_level(World.level(5), t_max=21) is CooperationLevel.ASYMMETRIC


def test_cooperation_level_classifies_level_6_as_mutual():
    assert lle.cooperation_level(World.level(6), t_max=21) is CooperationLevel.MUTUAL


def test_cooperation_level_trajectory_classifies_an_explicit_plan():
    world = World.level(3)
    plan = lle.solve(world, t_max=10)
    assert plan is not None

    assert lle.cooperation_level_trajectory(world, plan) is CooperationLevel.ASYMMETRIC


@pytest.mark.parametrize("level_idx", [1, 2])
def test_cooperation_level_independent_on_lle_levels_1_and_2(level_idx):
    assert lle.cooperation_level(World.level(level_idx), t_max=10) is CooperationLevel.INDEPENDENT


def test_cooperation_level_refines_is_cooperative():
    """is_cooperative(w) <=> cooperation_level(w) is in cooperative_subtypes()."""
    T_MAX = [10] * 4 + [21, 21]
    for level_idx, t_max in zip((1, 2, 3, 4, 6), T_MAX):
        world = World.level(level_idx)
        is_coop = lle.is_cooperative(world, t_max=t_max)
        precise = lle.cooperation_level(world, t_max=t_max)
        assert precise is not None
        assert is_coop == (precise in CooperationLevel.cooperative_subtypes())
        assert is_coop == precise.is_cooperative


def test_cooperation_level_unsolvable_for_walled_off_agent():
    assert lle.cooperation_level(World("S0 @ X"), t_max=10) is None


# ---------------------------------------------------------------------------
# is_cooperative
# ---------------------------------------------------------------------------


def test_is_cooperative_on_known_cooperative_level():
    # LLE Level 6 is canonically cooperative.
    world = World.level(6)
    assert lle.is_cooperative(world)


def test_is_cooperative_on_trivial_single_agent_level():
    world = World("S0 . X")
    assert not lle.is_cooperative(world, t_max=3)


def test_is_cooperative_on_explicitly_cooperative_worlds():
    worlds = [
        """
 . . L1S . X
S0 .  .  . .
S1 .  .  . .
 . .  .  . X""",
        """
.  . L0S . X
S0 .  .  . .
S1 .  .  . .
.  .  .  . X""",
        """
.   .  L0S . X
S0  .   .  . .
S1  .   .  . .
.  L1N  .  . X""",
        """
. L1S L0S .
S0  .   .  X
S1  .   .  X""",
    ]
    for ws in worlds:
        world = World(ws)
        assert lle.is_cooperative(world, 10)


def test_is_not_cooperative_on_independent_world():
    world = World("""
.  X L0S . X
S0 .  .  . .
S1 .  .  . .
.  X  .  . X
""")
    assert not lle.is_cooperative(world, 10)
