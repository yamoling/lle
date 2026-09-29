import pytest
from lle.characterization import WorldCharacterizer

from ...mocks import fail_if_called
from ...world_layouts import BLOCKED_UNSOLVABLE, LEVEL_1, LEVEL_6, SequentialCase, sequential_cases

SEQUENTIAL_CASES = sequential_cases()


@pytest.mark.parametrize("property_case", SEQUENTIAL_CASES, ids=lambda c: c.id)
def test_is_sequential(property_case: SequentialCase):
    """Check every catalogued positive and negative sequence-length expectation."""
    wc = WorldCharacterizer(property_case.layout.world(), property_case.t_max)

    assert wc.is_sequential(property_case.length) is property_case.expected


def test_is_sequential_rejects_length_below_2():
    """Sequence lengths below two are invalid."""
    wc = WorldCharacterizer(LEVEL_1.world(), t_max=10)

    for length in [1, 0, -1]:
        with pytest.raises(ValueError):
            wc.is_sequential(length)


def test_compute_shortest_path_without_sequence_rejects_length_below_2():
    wc = WorldCharacterizer(LEVEL_1.world(), t_max=10)

    for length in [1, 0, -1]:
        with pytest.raises(ValueError):
            wc.compute_shortest_path_without_sequence(length)


def test_unsolvable_world_is_not_sequential():
    """Sequential cooperation is undefined for an unsolvable world."""
    world = BLOCKED_UNSOLVABLE.world()
    assert not WorldCharacterizer(world, t_max=10).is_sequential(2)


def test_is_sequential_repeated_query_uses_cached_result(monkeypatch: pytest.MonkeyPatch):
    """A repeated length query returns without another universality solve."""
    wc = WorldCharacterizer(LEVEL_6.world(), t_max=21)
    first = wc.is_sequential(2)
    monkeypatch.setattr(wc, "compute_shortest_path_without_sequence", fail_if_called)
    assert wc.is_sequential(2) == first
    assert first


def test_is_sequential_uses_monotone_cache_inference():
    """True values infer shorter lengths and false values infer longer lengths."""
    true_cache = WorldCharacterizer(LEVEL_6.world(), t_max=21)
    true_cache._is_sequential_cache[3] = True
    false_cache = WorldCharacterizer(LEVEL_6.world(), t_max=21)
    false_cache._is_sequential_cache[2] = False
    assert true_cache.is_sequential(2)
    assert not false_cache.is_sequential(3)
    assert not false_cache.is_sequential(4)
