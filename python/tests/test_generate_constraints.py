"""Tests for WorldFilter and constrained world generation.

Section A: WorldFilter.is_satisfied_by — fast SAT checks against known levels.
Section B: the generation builder with filter shortcuts — integration tests (slow).
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
def test_generate_cooperative_shortcut_produces_cooperative_world():
    world = generate(width=5, height=5, n_agents=2).lasers(2).cooperative().build(max_attempts=500)
    assert world is not None
    assert Cooperative(20).is_satisfied_by(world)


@pytest.mark.slow
def test_generate_filter_object_same_as_shortcut():
    """An explicit WorldFilter via require() and the cooperative() shortcut both yield cooperative worlds."""
    t_max = 5 * 5 // 2
    world_shortcut = generate(width=5, height=5, n_agents=2).random().lasers(1).cooperative().build(max_attempts=500)
    world_obj = generate(width=5, height=5, n_agents=2).random().lasers(1).require(Cooperative(t_max)).build(max_attempts=500)
    assert world_shortcut is not None
    assert world_obj is not None
    assert Cooperative(t_max).is_satisfied_by(world_shortcut)
    assert Cooperative(t_max).is_satisfied_by(world_obj)


@pytest.mark.slow
def test_generate_take_all_satisfy_filter():
    """All worlds in a batch must satisfy the filter."""
    t_max = 5 * 5 // 2
    filter = Cooperative(t_max)
    builder = generate(width=5, height=5, n_agents=2).random().lasers(1).require(filter)
    for w in builder.take(3, n_jobs=1, max_attempts=500, progress=False):
        assert filter.is_satisfied_by(w)


@pytest.mark.slow
def test_generate_independent_produces_independent_world():
    world = generate().independent().build(seed=0, max_attempts=500)
    assert world is not None
    t_max = world.width * world.height // 2
    assert Independent(t_max).is_satisfied_by(world)


# ---------------------------------------------------------------------------
# The builder makes contradictory constraints unreachable: there is no way to
# ask for both an explicit filter and a shortcut at once, or for the
# contradictory cooperative=False + mutual=True. The last filter call wins.
# ---------------------------------------------------------------------------
def test_last_filter_call_wins():
    builder = generate(width=5, height=5, n_agents=2).cooperative().mutual()
    assert builder._world_filter.requires_mutual_cooperation
    builder = generate(width=5, height=5, n_agents=2).mutual().independent()
    assert not builder._world_filter.requires_cooperation


def test_require_overrides_named_filter():
    builder = generate(width=5, height=5, n_agents=2).cooperative().require(Solvable(21))
    assert isinstance(builder._world_filter, Solvable)


# ---------------------------------------------------------------------------
# Builder-level validation: generate().build() must raise for invalid combos
# ---------------------------------------------------------------------------


def test_generate_error_cooperative_n_agents_lt_2():
    with pytest.raises(ValueError, match="agents"):
        generate(width=5, height=5, n_agents=1).cooperative().build(max_attempts=1)


def test_generate_error_cooperative_n_lasers_0():
    with pytest.raises(ValueError, match="laser"):
        generate(width=5, height=5, n_agents=2).lasers(0).cooperative().build(max_attempts=1)


def test_generate_error_mutual_n_agents_lt_2():
    with pytest.raises(ValueError, match="agents"):
        generate(width=5, height=5, n_agents=1).mutual().build(max_attempts=1)


def test_generate_error_mutual_n_lasers_lt_2():
    with pytest.raises(ValueError, match="laser"):
        generate(width=5, height=5, n_agents=2).lasers(1).mutual().build(max_attempts=1)


def test_generate_error_chained_n_agents_lt_2():
    with pytest.raises(ValueError, match="agents"):
        generate(width=5, height=5, n_agents=1).chained().build(max_attempts=1)


def test_generate_error_chained_n_lasers_lt_2():
    with pytest.raises(ValueError, match="laser"):
        generate(width=5, height=5, n_agents=2).lasers(1).chained().build(max_attempts=1)


# ---------------------------------------------------------------------------
# Smart defaults: unset parameters get a sensible value from the filter
# ---------------------------------------------------------------------------


def test_default_lasers_chained_n_agents_2():
    """Exact example from the todo: must not raise ValueError about lasers.

    generate(width=5, height=5, n_agents=2).lanes().chained().build() was
    raising "Chained cooperation requires at least 2 lasers" because the
    auto-resolved count was n_agents-1 = 1.  The fix defaults to min(n_agents, 2).
    """
    builder = generate(width=5, height=5, n_agents=2).lanes().chained()
    placement = builder._resolve_placement(builder._starts)
    n_lasers = builder._resolve_n_lasers(placement)
    assert n_lasers >= 2, f"Expected auto n_lasers >= 2 for chained filter with n_agents=2, got {n_lasers}"


def test_default_lasers_mutual_n_agents_2():
    """Same default-value fix applies to the mutual filter (mentioned in the todo).

    generate(width=5, height=5, n_agents=2).mutual().build() would also have
    raised without the fix because mutual implies chained cooperation.
    """
    builder = generate(width=5, height=5, n_agents=2).mutual()
    placement = builder._resolve_placement(builder._starts)
    n_lasers = builder._resolve_n_lasers(placement)
    assert n_lasers >= 2, f"Expected auto n_lasers >= 2 for mutual filter with n_agents=2, got {n_lasers}"


def test_default_lasers_chained_does_not_raise_on_build():
    """End-to-end: generate().lanes().chained().build() must not raise ValueError."""
    generate(width=5, height=5, n_agents=2).lanes().chained().build(max_attempts=1)


def test_default_lasers_mutual_does_not_raise_on_build():
    """End-to-end: generate().mutual().build() must not raise ValueError."""
    generate(width=5, height=5, n_agents=2).mutual().build(max_attempts=1)


def test_explicit_laser_count_overrides_default():
    """An explicit lasers() call must always win over the smart default."""
    builder = generate(width=5, height=5, n_agents=3).lasers(3).chained()
    placement = builder._resolve_placement(builder._starts)
    n_lasers = builder._resolve_n_lasers(placement)
    assert n_lasers == 3
