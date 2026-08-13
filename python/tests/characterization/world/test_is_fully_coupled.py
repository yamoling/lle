import pytest
from lle.characterization import WorldCharacterizer

from ...world_layouts import ScalarPropertyCase, scalar_cases_for


@pytest.mark.parametrize("property_case", scalar_cases_for("fully_coupled"), ids=lambda case: case.id)
def test_is_fully_coupled_matches_catalog(property_case: ScalarPropertyCase):
    characterizer = WorldCharacterizer(property_case.layout.world(), property_case.t_max)
    assert characterizer.is_fully_coupled() is property_case.expected
