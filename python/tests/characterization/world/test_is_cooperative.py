import pytest
from lle.characterization import WorldCharacterizer

from .layouts import (
    BLOCKED_UNSOLVABLE,
    ONE_WAY_DETOUR,
    ScalarPropertyCase,
    scalar_cases_for,
)

COOPERATIVE_CASES = scalar_cases_for("cooperative")


@pytest.mark.parametrize("case", COOPERATIVE_CASES, ids=lambda case: case.id)
def test_is_cooperative(case: ScalarPropertyCase):
    """Check whether cooperation is required at each catalog horizon.

    A longer horizon can make a detour reachable and remove the need for one
    agent to block a laser for another.

    @ai-generated
    """
    characterizer = WorldCharacterizer(case.layout.world(), case.t_max)

    assert characterizer.is_cooperative() is case.expected


def test_one_way_detour_stops_requiring_cooperation_at_t10():
    """The longer independent route becomes available exactly at t=10.

    @ai-generated
    """
    before_detour = WorldCharacterizer(ONE_WAY_DETOUR.world(), t_max=9)
    after_detour = WorldCharacterizer(ONE_WAY_DETOUR.world(), t_max=10)

    assert before_detour.is_cooperative()
    assert not after_detour.is_cooperative()


def test_unsolvable_world_raises_on_is_cooperative():
    """Reject cooperation queries when the catalog world has no solution.

    @ai-generated
    """
    world = BLOCKED_UNSOLVABLE.world()

    with pytest.raises(ValueError):
        WorldCharacterizer(world, t_max=10).is_cooperative()
