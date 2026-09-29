from lle.characterization.monotone_cache import MonotoneCache


def test_monotone_cache_only_infers_negative_results_for_higher_steps():
    cache = MonotoneCache()
    for i in range(10):
        assert cache.get(i) is None
    cache[5] = False
    for i in range(5, 10):
        value = cache.get(i)
        assert value is not None and not value

    for i in range(5):
        value = cache.get(i)
        assert value is None


def test_monotone_cache_only_infers_positive_results_for_lower_steps():
    cache = MonotoneCache()
    for i in range(10):
        assert cache.get(i) is None
    cache[5] = True
    for i in range(6, 10):
        value = cache.get(i)
        assert value is None

    for i in range(5):
        value = cache.get(i)
        assert value is not None and value
