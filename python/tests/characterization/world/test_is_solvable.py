import pytest
from lle.characterization import WorldCharacterizer

from ...world_layouts import ScalarPropertyCase, scalar_cases_for

SOLVABLE_CASES = scalar_cases_for("solvable")


@pytest.mark.parametrize("property_case", SOLVABLE_CASES, ids=lambda c: c.id)
def test_is_solvable(property_case: ScalarPropertyCase):
    """Check every catalogued positive and negative solvability expectation.

    @ai-generated
    """
    wc = WorldCharacterizer(property_case.layout.world(), property_case.t_max)

    assert wc.is_solvable() is property_case.expected
