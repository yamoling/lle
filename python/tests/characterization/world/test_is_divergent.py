from __future__ import annotations

import lle
import lle.characterization as characterization
import pytest
from lle.characterization import WorldCharacterizer

from ...mocks import fail_if_called
from ...world_layouts import (
    DIVERGENT_2_TIGHT,
    DIVERGENT_2_WITH_DETOUR,
    LEVEL_6,
    OPEN_TWO_AGENT,
    UNSOLVABLE_4AGENTS,
    DivergentCase,
    divergent_cases,
)


@pytest.mark.parametrize("property_case", divergent_cases(), ids=lambda case: case.id)
def test_is_divergent_matches_catalogue(property_case: DivergentCase):
    wc = WorldCharacterizer(property_case.layout.world(), property_case.t_max)
    assert wc.is_divergent(property_case.k) == property_case.expected


@pytest.mark.parametrize("k", [-1, 0, 1])
def test_divergence_public_entry_points_reject_threshold_below_two(k: int):
    world = OPEN_TWO_AGENT.world()
    wc = WorldCharacterizer(world, t_max=6)
    helpers = (
        lambda: wc.is_divergent(k),
        lambda: wc.compute_shortest_path_without_divergence(k),
        lambda: lle.is_divergent(world, t_max=6, k=k),
        lambda: characterization.is_divergent(world, t_max=6, k=k),
    )
    for helper in helpers:
        with pytest.raises(ValueError):
            helper()


def test_unsolvable_world_is_not_divergent():
    assert not WorldCharacterizer(UNSOLVABLE_4AGENTS.world(), t_max=10).is_divergent(2)


def test_is_divergent_repeated_query_uses_cached_result(monkeypatch: pytest.MonkeyPatch):
    """A repeated threshold query returns without another universality solve."""
    wc = WorldCharacterizer(DIVERGENT_2_TIGHT.world(), t_max=2)
    first = wc.is_divergent(2)
    monkeypatch.setattr(wc, "compute_shortest_path_without_divergence", fail_if_called)
    assert wc.is_divergent(2) == first
    assert first


def test_is_divergent_uses_monotone_cache_inference():
    true_cache = WorldCharacterizer(LEVEL_6.world(), t_max=6)
    true_cache._is_divergent_cache[3] = True
    false_cache = WorldCharacterizer(LEVEL_6.world(), t_max=6)
    false_cache._is_divergent_cache[2] = False
    assert true_cache.is_divergent(2)
    assert not false_cache.is_divergent(3)
    assert not false_cache.is_divergent(4)


def test_required_divergence_needs_the_avoiding_solve_to_be_unsat():
    required = WorldCharacterizer(DIVERGENT_2_TIGHT.world(), t_max=2)
    assert required.is_divergent(2)
    assert required.compute_shortest_path_without_divergence(2) is None

    avoidable = WorldCharacterizer(DIVERGENT_2_WITH_DETOUR.world(), t_max=6)
    assert avoidable.compute_shortest_path_without_divergence(2) is not None
    assert not avoidable.is_divergent(2)


def test_required_divergence_is_horizon_dependent():
    """The detour layout stops requiring divergence exactly when the detour fits."""
    world = DIVERGENT_2_WITH_DETOUR.world
    assert WorldCharacterizer(world(), t_max=5).is_divergent(2)
    assert not WorldCharacterizer(world(), t_max=6).is_divergent(2)
