from __future__ import annotations

import lle
import lle.characterization as characterization
import pytest
from lle.characterization import WorldCharacterizer
from lle.characterization.world_characterization import NotSolvableError

from ...mocks import fail_if_called
from ...world_layouts import (
    BLOCKED_UNSOLVABLE,
    CONVERGENT_2_TIGHT,
    OPEN_TWO_AGENT,
    ConvergentCase,
    convergent_cases,
)


@pytest.mark.parametrize("property_case", convergent_cases(), ids=lambda case: case.id)
def test_is_convergent_matches_catalogue(property_case: ConvergentCase):
    """Check every catalogued convergence-threshold expectation."""
    wc = WorldCharacterizer(property_case.layout.world(), property_case.t_max)
    assert wc.is_convergent(property_case.k) == property_case.expected


@pytest.mark.parametrize("k", [-1, 0, 1])
def test_convergence_public_entry_points_reject_threshold_below_two(k: int):
    """All Python characterization entry points reject invalid thresholds."""
    world = OPEN_TWO_AGENT.world()
    wc = WorldCharacterizer(world, t_max=6)
    helpers = (
        lambda: wc.is_convergent(k),
        lambda: wc.compute_shortest_path_without_convergence(k),
        lambda: lle.is_convergent(world, t_max=6, k=k),
        lambda: characterization.is_convergent(world, t_max=6, k=k),
    )
    for helper in helpers:
        with pytest.raises(ValueError):
            helper()


def test_unsolvable_world_raises_on_is_convergent():
    """Convergence is undefined for an unsolvable world."""
    with pytest.raises(NotSolvableError):
        WorldCharacterizer(BLOCKED_UNSOLVABLE.world(), t_max=10).is_convergent(2)


def test_is_convergent_repeated_query_uses_cached_result(monkeypatch: pytest.MonkeyPatch):
    """A repeated threshold query returns without another universality solve."""
    wc = WorldCharacterizer(CONVERGENT_2_TIGHT.world(), t_max=5)
    first = wc.is_convergent(2)
    monkeypatch.setattr(wc, "compute_shortest_path_without_convergence", fail_if_called)
    assert wc.is_convergent(2) == first
    assert first


def test_is_convergent_uses_monotone_cache_inference():
    """True values infer lower thresholds and false values infer higher thresholds."""
    true_cache = WorldCharacterizer(OPEN_TWO_AGENT.world(), t_max=6)
    true_cache._is_convergent_cache[3] = True
    false_cache = WorldCharacterizer(OPEN_TWO_AGENT.world(), t_max=6)
    false_cache._is_convergent_cache[2] = False
    assert true_cache.is_convergent(2)
    assert not false_cache.is_convergent(3)
    assert not false_cache.is_convergent(4)


def test_normal_witness_without_convergence_short_circuits_universality(monkeypatch: pytest.MonkeyPatch):
    """One non-convergent normal solution proves convergence is not required."""
    wc = WorldCharacterizer(OPEN_TWO_AGENT.world(), t_max=6)
    monkeypatch.setattr(wc, "compute_shortest_path_without_convergence", fail_if_called)
    assert not wc.is_convergent(2)


def test_is_convergent_is_exported_from_both_public_paths():
    """The package root and characterization module expose the convenience helper."""
    assert lle.is_convergent is characterization.is_convergent
