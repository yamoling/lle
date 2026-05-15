"""Tests for lle.cooperation_level and the CooperationLevel enum."""

from __future__ import annotations

import pytest

import lle
from lle import CooperationLevel, World


# ---------------------------------------------------------------------------
# Enum invariants
# ---------------------------------------------------------------------------

def test_cooperation_level_is_str_compatible():
    assert CooperationLevel.COOPERATIVE == "cooperative"
    assert CooperationLevel.FULLY_COUPLED == "fully_coupled"
    assert "asymmetric" == CooperationLevel.ASYMMETRIC


def test_cooperation_level_round_trips_through_string():
    for level in CooperationLevel:
        assert CooperationLevel(level.value) is level


def test_cooperative_subtypes_excludes_unsolvable_and_independent():
    subtypes = CooperationLevel.cooperative_subtypes()
    assert CooperationLevel.UNSOLVABLE not in subtypes
    assert CooperationLevel.INDEPENDENT not in subtypes
    assert set(subtypes) == set(CooperationLevel) - {
        CooperationLevel.UNSOLVABLE,
        CooperationLevel.INDEPENDENT,
    }


# ---------------------------------------------------------------------------
# Public function against canonical LLE levels
# ---------------------------------------------------------------------------

def test_cooperation_level_returns_enum_member():
    world = World.level(1)
    assert isinstance(lle.cooperation_level(world), CooperationLevel)


def test_cooperation_level_independent_for_trivial_world():
    assert lle.cooperation_level(World("S0 . X"), t_max=5) is CooperationLevel.INDEPENDENT


def test_cooperation_level_classifies_level_6_as_cooperative_subtype():
    level = lle.cooperation_level(World.level(6))
    assert level in CooperationLevel.cooperative_subtypes()


@pytest.mark.parametrize("level_idx", [1, 2])
def test_cooperation_level_independent_on_lle_levels_1_and_2(level_idx):
    assert lle.cooperation_level(World.level(level_idx), t_max=10) is CooperationLevel.INDEPENDENT


def test_cooperation_level_refines_is_cooperative():
    """is_cooperative(w) <=> cooperation_level(w) is in cooperative_subtypes()."""
    for level_idx in (1, 2, 3, 4, 6):
        world = World.level(level_idx)
        is_coop = lle.is_cooperative(world, t_max=25)
        precise = lle.cooperation_level(world, t_max=25)
        assert is_coop == (precise in CooperationLevel.cooperative_subtypes())


def test_cooperation_level_unsolvable_for_walled_off_agent():
    assert lle.cooperation_level(World("S0 @ X"), t_max=10) is CooperationLevel.UNSOLVABLE


# ---------------------------------------------------------------------------
# Generator profile parameter
# ---------------------------------------------------------------------------

def test_generator_profile_requires_cooperative():
    with pytest.raises(ValueError, match="only meaningful when cooperative=True"):
        lle.generate(
            kind="random",
            width=5,
            height=5,
            n_agents=2,
            cooperative=False,
            profile=CooperationLevel.MUTUAL,
            seed=0,
        )


def test_generator_rejects_non_cooperative_subtype_profile():
    with pytest.raises(ValueError, match="profile must be one of"):
        lle.generate(
            kind="random",
            width=5,
            height=5,
            n_agents=2,
            cooperative=True,
            profile=CooperationLevel.INDEPENDENT,
            seed=0,
        )


def test_generator_produces_world_matching_requested_profile():
    """Sanity check: constructive cooperative with profile=ASYMMETRIC yields
    a world that classifies as ASYMMETRIC. The lane-based structural laser
    produces a single helper -> beneficiary dependency, which is the
    asymmetric profile by definition."""
    world = lle.generate(
        kind="constructive",
        width=6,
        height=6,
        n_agents=2,
        n_lasers=1,
        cooperative=True,
        profile=CooperationLevel.ASYMMETRIC,
        t_max=15,
        seed=0,
        max_attempts=50,
    )
    assert world is not None
    assert lle.cooperation_level(world, t_max=15) is CooperationLevel.ASYMMETRIC
