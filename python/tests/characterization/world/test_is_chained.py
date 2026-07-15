import pytest
from lle.characterization import WorldCharacterizer

from ...world_layouts import BLOCKED_UNSOLVABLE, LEVEL_1, ChainedCase, chained_cases

CHAINED_CASES = chained_cases()


@pytest.mark.parametrize("property_case", CHAINED_CASES, ids=lambda c: c.id)
def test_is_chained(property_case: ChainedCase):
    """Check every catalogued positive and negative chain-length expectation."""
    wc = WorldCharacterizer(property_case.layout.world(), property_case.t_max)

    assert wc.is_chained(property_case.length) is property_case.expected


def test_is_chained_rejects_length_below_2():
    """Chain lengths below two are invalid."""
    wc = WorldCharacterizer(LEVEL_1.world(), t_max=10)

    for length in [1, 0, -1]:
        with pytest.raises(ValueError):
            wc.is_chained(length)


def test_compute_shortest_path_without_chain_rejects_length_below_2():
    wc = WorldCharacterizer(LEVEL_1.world(), t_max=10)

    for length in [1, 0, -1]:
        with pytest.raises(ValueError):
            wc.compute_shortest_path_without_chain(length)


def test_unsolvable_world_raises_on_is_chained():
    """Chained cooperation is undefined for an unsolvable world."""
    world = BLOCKED_UNSOLVABLE.world()

    with pytest.raises(ValueError):
        WorldCharacterizer(world, t_max=10).is_chained(2)
