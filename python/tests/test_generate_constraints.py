"""Tests for WorldFilter and constrained world generation.

Section A: WorldFilter.is_satisfied_by — fast SAT checks against known levels.
Section B: generate() with filter/cooperative/mutual — integration tests (slow).
Section C: backward-compatibility — existing call patterns still work.
"""

from __future__ import annotations

import pytest
from lle import World
from lle.generator import WorldFilter, generate

# ---------------------------------------------------------------------------
# A. WorldFilter.is_satisfied_by — unit tests
# ---------------------------------------------------------------------------


def test_filter_no_constraints_accepts_solvable_world():
    world = World("S0 . S1\n.  .  .\nX  .  X")
    assert WorldFilter().is_satisfied_by(world, default_t_max=10)


def test_filter_no_constraints_rejects_unsolvable_world():
    world = World("S0 @ X")
    assert not WorldFilter().is_satisfied_by(world, default_t_max=10)


def test_filter_cooperative_true_accepts_level6():
    """Level 6 requires cooperation — cooperative=True should accept it."""
    assert WorldFilter(cooperative=True).is_satisfied_by(World.level(6), default_t_max=21)


def test_filter_cooperative_false_rejects_level6():
    """Level 6 requires cooperation — cooperative=False should reject it."""
    assert not WorldFilter(cooperative=False).is_satisfied_by(World.level(6), default_t_max=21)


def test_filter_cooperative_false_accepts_level1():
    """Level 1 is independently solvable — cooperative=False should accept it."""
    assert WorldFilter(cooperative=False).is_satisfied_by(World.level(1), default_t_max=10)


def test_filter_cooperative_true_rejects_level1():
    """Level 1 needs no cooperation — cooperative=True should reject it."""
    assert not WorldFilter(cooperative=True).is_satisfied_by(World.level(1), default_t_max=10)


def test_filter_mutual_true_accepts_level6():
    assert WorldFilter(mutual=True).is_satisfied_by(World.level(6), default_t_max=21)


def test_filter_mutual_true_rejects_level3():
    """Level 3 requires cooperation but not mutual — mutual=True should reject it."""
    assert not WorldFilter(mutual=True).is_satisfied_by(World.level(3), default_t_max=21)


def test_filter_mutual_false_accepts_level3():
    """Level 3 is cooperative but not mutual — mutual=False should accept it."""
    assert WorldFilter(mutual=False).is_satisfied_by(World.level(3), default_t_max=21)


def test_filter_cooperative_true_mutual_false_accepts_level3():
    """Level 3 is cooperative but not mutual — both constraints together should accept."""
    assert WorldFilter(cooperative=True, mutual=False).is_satisfied_by(World.level(3), default_t_max=21)


def test_filter_t_max_override_rejects_when_too_short():
    """Level 6 needs 21 steps; restricting to t_max=20 makes it unsolvable."""
    # cooperative=True should fail because with t_max=20 the world is unsolvable
    wf = WorldFilter(cooperative=True, t_max=20)
    assert not wf.is_satisfied_by(World.level(6), default_t_max=21)


def test_filter_uses_default_t_max_when_none():
    """Without an explicit t_max the filter uses the default passed in."""
    wf = WorldFilter(cooperative=True)
    # t_max=21 → level 6 is solvable and cooperative → True
    assert wf.is_satisfied_by(World.level(6), default_t_max=21)
    # t_max=20 → level 6 is unsolvable → False
    assert not wf.is_satisfied_by(World.level(6), default_t_max=20)


def test_filter_mutual_true_rejects_level6_with_insufficient_t_max():
    """Level 6 needs exactly 21 steps; t_max=20 makes it unsolvable → mutual=True fails."""
    assert not WorldFilter(mutual=True).is_satisfied_by(World.level(6), default_t_max=20)


# ---------------------------------------------------------------------------
# B. generate() with filter / cooperative / mutual — integration (slow)
# ---------------------------------------------------------------------------

_SMALL = dict(kind="random", height=5, width=5, n_agents=2, n_lasers=1)


@pytest.mark.slow
def test_generate_cooperation_kwarg_produces_cooperative_world():
    world = generate(cooperative=True)
    generate()
    assert world is not None
    t_max = world.width * world.height // 2
    assert WorldFilter(cooperative=True).is_satisfied_by(world, t_max)


@pytest.mark.slow
def test_generate_filter_object_same_as_kwarg():
    """WorldFilter object and cooperation= kwarg should produce identically-filtered worlds."""
    t_max = 5 * 5 // 2
    world_kw = generate(cooperative=True)
    generate(t_max=10)
    filter = WorldFilter(True)
    generate(filter=filter)
    world_obj = generate(filter=WorldFilter(cooperative=True), t_max=t_max)
    assert WorldFilter(cooperative=True).is_satisfied_by(world_kw, t_max)
    assert WorldFilter(cooperative=True).is_satisfied_by(world_obj, t_max)


@pytest.mark.slow
def test_generate_n_all_satisfy_filter():
    """All worlds in a batch must satisfy the filter."""
    t_max = 5 * 5 // 2
    filter = WorldFilter(cooperative=True)
    for w in generate(n=3, filter=filter, max_attempts=500):
        assert filter.is_satisfied_by(w, t_max)


@pytest.mark.slow
def test_generate_cooperation_false_produces_independent_world():
    world = generate(cooperative=False, seed=0)
    assert world is not None
    t_max = world.width * world.height // 2
    assert WorldFilter(cooperative=False).is_satisfied_by(world, t_max)


@pytest.mark.slow
def test_generate_mutual_kwarg():
    """mutual=True: returned world (if any) must require mutual cooperation."""
    world = generate(mutual=True, kind="random", height=7, width=7, n_agents=2, n_lasers=1, seed=0, max_attempts=50)
    if world is None:
        pytest.skip("budget exhausted — no mutual world found within 500 attempts")
    t_max = 7 * 7 // 2
    assert WorldFilter(mutual=True).is_satisfied_by(world, t_max)


# ---------------------------------------------------------------------------
# Error cases
# ---------------------------------------------------------------------------
@pytest.mark.parametrize("kwargs", ({"cooperative": True, "filter": WorldFilter()}, {"mutual": False, "filter": WorldFilter}))
def test_wrong_args_raises_exception(kwargs: dict):
    with pytest.raises(ValueError, match="Cannot combine"):
        generate(**kwargs)
