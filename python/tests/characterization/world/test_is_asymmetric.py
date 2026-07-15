import pytest
from lle.characterization import WorldCharacterizer

from .layouts import BLOCKED_UNSOLVABLE, ScalarPropertyCase, scalar_cases_for

ASYMMETRIC_CASES = scalar_cases_for("asymmetric")


@pytest.mark.parametrize("case", ASYMMETRIC_CASES, ids=lambda case: case.id)
def test_is_asymmetric(case: ScalarPropertyCase):
    """Check whether asymmetric help is required at each catalog horizon.

    Longer horizons may admit an independent detour or reciprocal help, making a
    world no longer asymmetric.
    """
    characterizer = WorldCharacterizer(case.layout.world(), case.t_max)
    assert characterizer.is_asymmetric() == case.expected


def test_unsolvable_world_raises_on_is_asymmetric():
    """Reject asymmetry queries when the catalog world has no solution.

    @ai-generated
    """
    world = BLOCKED_UNSOLVABLE.world()

    with pytest.raises(ValueError):
        WorldCharacterizer(world, t_max=10).is_asymmetric()
