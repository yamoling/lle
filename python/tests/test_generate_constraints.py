"""Tests for WorldFilter and constrained world generation.

Section A: WorldFilter.is_satisfied_by — fast SAT checks against known levels.
Section B: generate() with filter/cooperative/mutual — integration tests (slow).
Section C: backward-compatibility — existing call patterns still work.
"""

from __future__ import annotations

import pytest
from lle import World
from lle.generator import Cooperative, Independent, Mutual, Solvable, WorldFilter, generate

# ---------------------------------------------------------------------------
# A. WorldFilter.is_satisfied_by — unit tests
# ---------------------------------------------------------------------------


def test_filter_no_constraints_accepts_solvable_world():
    world = World("S0 . S1\n.  .  .\nX  .  X")
    assert Solvable(10).is_satisfied_by(world)


def test_filter_no_constraints_rejects_unsolvable_world():
    world = World("S0 @ X")
    assert not Solvable(10).is_satisfied_by(world)


def test_filter_cooperative_accepts_level6():
    """Level 6 requires cooperation — Cooperative should accept it."""
    assert Cooperative(21).is_satisfied_by(World.level(6))


def test_filter_independent_rejects_level6():
    """Level 6 requires cooperation — Independent should reject it."""
    assert not Independent(21).is_satisfied_by(World.level(6))


def test_filter_independent_accepts_level1():
    """Level 1 is independently solvable — Independent should accept it."""
    assert Independent(10).is_satisfied_by(World.level(1))


def test_filter_cooperative_rejects_level1():
    """Level 1 needs no cooperation — Cooperative should reject it."""
    assert not Cooperative(10).is_satisfied_by(World.level(1))


def test_filter_mutual_accepts_level6():
    assert Mutual(21).is_satisfied_by(World.level(6))


def test_filter_mutual_rejects_level3():
    """Level 3 requires cooperation but not mutual — Mutual should reject it."""
    assert not Mutual(21).is_satisfied_by(World.level(3))


def test_filter_cooperative_accepts_non_mutual_level3():
    """Level 3 is cooperative (though not mutual) — Cooperative should accept it."""
    assert Cooperative(21).is_satisfied_by(World.level(3))


def test_filter_t_max_override_rejects_when_too_short():
    """Level 6 needs 21 steps; restricting to t_max=20 makes it unsolvable."""
    wf = Cooperative(20)
    assert not wf.is_satisfied_by(World.level(6))


def test_filter_uses_default_t_max_when_none():
    """Without an explicit t_max the filter uses the default passed in."""
    wf = Cooperative(21)
    # t_max=21 → level 6 is solvable and cooperative → True
    assert wf.is_satisfied_by(World.level(6))
    # t_max=20 → level 6 is unsolvable → False
    wf = Cooperative(20)
    assert not wf.is_satisfied_by(World.level(6))


def test_filter_mutual_rejects_level6_with_insufficient_t_max():
    """Level 6 needs exactly 21 steps; t_max=20 makes it unsolvable → Mutual fails."""
    assert not Mutual(20).is_satisfied_by(World.level(6))


def test_world_filter_is_abstract():
    """The base WorldFilter cannot be instantiated; use a concrete subclass."""
    with pytest.raises(TypeError):
        WorldFilter(10)  # pyright: ignore[reportAbstractUsage]


def test_mutual_is_a_cooperative_filter():
    """Mutual cooperation implies cooperation — the hierarchy must reflect that."""
    assert isinstance(Mutual(10), Cooperative)
    assert Cooperative(10).requires_cooperation
    assert Mutual(10).requires_cooperation
    assert not Independent(10).requires_cooperation
    assert not Solvable(10).requires_cooperation


@pytest.mark.slow
def test_generate_cooperation_kwarg_produces_cooperative_world():
    world = generate(width=5, height=5, cooperative=True, kind="constructive", n_lasers=2, n_agents=2)
    assert world is not None
    assert Cooperative(20).is_satisfied_by(world)


@pytest.mark.slow
def test_generate_filter_object_same_as_kwarg():
    """A WorldFilter object and the cooperative= kwarg produce identically-filtered worlds."""
    t_max = 5 * 5 // 2
    world_kw = generate(kind="random", height=5, width=5, n_agents=2, n_lasers=1, cooperative=True)
    world_obj = generate(kind="random", height=5, width=5, n_agents=2, n_lasers=1, filter=Cooperative(t_max))
    assert world_obj is not None
    assert Cooperative(t_max).is_satisfied_by(world_kw)
    assert Cooperative(t_max).is_satisfied_by(world_obj)


@pytest.mark.slow
def test_generate_n_all_satisfy_filter():
    """All worlds in a batch must satisfy the filter."""
    t_max = 5 * 5 // 2
    filter = Cooperative(t_max)
    for w in generate(n=3, n_jobs=1, kind="random", height=5, width=5, n_agents=2, n_lasers=1, filter=filter, max_attempts=500):
        assert filter.is_satisfied_by(w)


@pytest.mark.slow
def test_generate_cooperation_false_produces_independent_world():
    world = generate(cooperative=False, seed=0)
    assert world is not None
    t_max = world.width * world.height // 2
    assert Independent(t_max).is_satisfied_by(world)


# ---------------------------------------------------------------------------
# Error cases
# ---------------------------------------------------------------------------
@pytest.mark.parametrize("kwargs", ({"cooperative": True, "filter": Solvable(21)}, {"mutual": False, "filter": Solvable(21)}))
def test_combining_filter_and_shortcut_raises(kwargs: dict):
    with pytest.raises(ValueError, match="Cannot combine"):
        generate(**kwargs)


def test_contradictory_shortcuts_raise():
    with pytest.raises(ValueError, match="contradictory"):
        generate(cooperative=False, mutual=True)
