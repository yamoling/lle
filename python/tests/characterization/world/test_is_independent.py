import pytest
from lle.characterization import WorldCharacterizer

from ...world_layouts import BLOCKED_UNSOLVABLE, ScalarPropertyCase, scalar_cases_for

INDEPENDENT_CASES = scalar_cases_for("independent")


@pytest.mark.parametrize("case", INDEPENDENT_CASES, ids=lambda case: case.id)
def test_is_independent(case: ScalarPropertyCase):
    """Check whether an independent solution exists at each catalog horizon.

    Open worlds are independent, while laser layouts can become independent only
    after the horizon is long enough to permit a non-blocking detour.
    """
    characterizer = WorldCharacterizer(case.layout.world(), case.t_max)
    assert characterizer.is_independent() is case.expected


def test_unsolvable_world_is_not_independent():
    """Reject independence queries when the catalog world has no solution."""
    world = BLOCKED_UNSOLVABLE.world()
    assert not WorldCharacterizer(world, t_max=10).is_independent()
