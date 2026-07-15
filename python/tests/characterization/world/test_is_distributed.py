import pytest
from lle.characterization import WorldCharacterizer

from ...world_layouts import DistributedCase, distributed_cases


@pytest.mark.parametrize("property_case", distributed_cases(), ids=lambda case: case.id)
@pytest.mark.xfail(
    strict=True,
    raises=NotImplementedError,
    reason="WorldCharacterizer.is_distributed is not implemented yet",
)
def test_is_distributed_matches_catalog(property_case: DistributedCase):
    """Specify distributed characterization for every catalogued expectation.

    @ai-generated
    """
    characterizer = WorldCharacterizer(property_case.layout.world(), property_case.t_max)

    assert characterizer.is_distributed(property_case.order) is property_case.expected
