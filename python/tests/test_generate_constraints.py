"""Tests for predicate constraints and constrained world generation.

Section A: Constraint.is_satisfied_by — fast SAT checks against known levels.
Section B: the generation builder with predicate shortcuts — integration tests.
"""

from __future__ import annotations

import pytest
from lle import World
from lle.generator import Chained, Constraint, Cooperative, Independent, Interdependent, Mutual, Solvable, WorldRequirements, generate

# ---------------------------------------------------------------------------
# A. Constraint.is_satisfied_by — unit tests
# ---------------------------------------------------------------------------


def test_filter_no_constraints_accepts_solvable_world():
    world = World("S0 . S1\n.  .  .\nX  .  X")
    assert Constraint(10).is_satisfied_by(world)


def test_filter_no_constraints_rejects_unsolvable_world():
    world = World("S0 @ X")
    assert not Constraint(10).is_satisfied_by(world)


def test_filter_cooperative_accepts_level6():
    """Level 6 requires cooperation — Cooperative should accept it."""
    assert Constraint(21, Cooperative()).is_satisfied_by(World.level(6))


def test_filter_independent_rejects_level6():
    """Level 6 requires cooperation — Independent should reject it."""
    assert not Constraint(21, Independent()).is_satisfied_by(World.level(6))


def test_filter_independent_accepts_level1():
    """Level 1 is independently solvable — Independent should accept it."""
    assert Constraint(10, Independent()).is_satisfied_by(World.level(1))


def test_filter_cooperative_rejects_level1():
    """Level 1 needs no cooperation — Cooperative should reject it."""
    assert not Constraint(10, Cooperative()).is_satisfied_by(World.level(1))


def test_filter_mutual_accepts_level6():
    assert Constraint(21, Mutual()).is_satisfied_by(World.level(6))


def test_filter_mutual_rejects_level3():
    """Level 3 requires cooperation but not mutual — Mutual should reject it."""
    assert not Constraint(21, Mutual()).is_satisfied_by(World.level(3))


def test_filter_cooperative_accepts_non_mutual_level3():
    """Level 3 is cooperative (though not mutual) — Cooperative should accept it."""
    assert Constraint(21, Cooperative()).is_satisfied_by(World.level(3))


def test_filter_t_max_override_rejects_when_too_short():
    """Level 6 needs 21 steps; restricting to t_max=20 makes it unsolvable."""
    constraint = Constraint(20, Cooperative())
    assert not constraint.is_satisfied_by(World.level(6))


def test_constraint_uses_its_t_max():
    constraint = Constraint(21, Cooperative())
    # t_max=21 → level 6 is solvable and cooperative → True
    assert constraint.is_satisfied_by(World.level(6))
    # t_max=20 → level 6 is unsolvable → False
    constraint = Constraint(20, Cooperative())
    assert not constraint.is_satisfied_by(World.level(6))


def test_filter_mutual_rejects_level6_with_insufficient_t_max():
    """Level 6 needs exactly 21 steps; t_max=20 makes it unsolvable → Mutual fails."""
    assert not Constraint(20, Mutual()).is_satisfied_by(World.level(6))


def test_world_requirements_for_atoms():
    assert Solvable().requirements == WorldRequirements()
    assert Independent().requirements == WorldRequirements()
    assert Cooperative().requirements == WorldRequirements(min_lasers=1, min_agents=2, cooperation=True)
    assert Mutual().requirements == WorldRequirements(min_lasers=2, min_agents=2, cooperation=True, mutual=True)


def test_world_requirements_compose_over_boolean_expressions():
    assert (Chained(3) & ~Mutual()).requirements == WorldRequirements(min_lasers=3, min_agents=2, cooperation=True)
    assert (Mutual() | Interdependent(5)).requirements == WorldRequirements(min_lasers=2, min_agents=2, cooperation=True, mutual=True)
    assert (~Interdependent(5)).requirements == WorldRequirements()


def test_generate_cooperative_shortcut_produces_cooperative_world():
    world = generate(width=5, height=5, n_agents=2).lasers(2).cooperative().build(max_attempts=500)
    assert world is not None
    assert Constraint(20, Cooperative()).is_satisfied_by(world)


def test_generate_filter_object_same_as_shortcut():
    """An explicit predicate via require() and the cooperative() shortcut both yield cooperative worlds."""
    t_max = 5 * 5 // 2
    world_shortcut = generate(width=5, height=5, n_agents=2).random().lasers(1).cooperative().build(max_attempts=500)
    world_obj = generate(width=5, height=5, n_agents=2).random().lasers(1).require(Cooperative()).build(max_attempts=500)
    constraint = Constraint(t_max, Cooperative())
    assert world_shortcut is not None
    assert world_obj is not None
    assert constraint.is_satisfied_by(world_shortcut)
    assert constraint.is_satisfied_by(world_obj)


def test_generate_take_all_satisfy_filter():
    """All worlds in a batch must satisfy the constraint."""
    t_max = 5 * 5 // 2
    predicate = Cooperative()
    constraint = Constraint(t_max, predicate)
    builder = generate(width=5, height=5, n_agents=2).random().lasers(1).require(predicate)
    for w in builder.take(3, n_jobs=1, max_attempts=500, progress=False):
        assert constraint.is_satisfied_by(w)


def test_generate_independent_produces_independent_world():
    world = generate().independent().build(seed=0, max_attempts=500)
    assert world is not None
    t_max = world.width * world.height // 2
    assert Constraint(t_max, Independent()).is_satisfied_by(world)


# ---------------------------------------------------------------------------
# The last predicate-setting call wins.
# ---------------------------------------------------------------------------
def test_last_filter_call_wins():
    builder = generate(width=5, height=5, n_agents=2).cooperative().mutual()
    assert isinstance(builder._constraint.predicate, Mutual)
    builder = generate(width=5, height=5, n_agents=2).mutual().independent()
    assert isinstance(builder._constraint.predicate, Independent)


def test_require_overrides_named_filter():
    builder = generate(width=5, height=5, n_agents=2).cooperative().require(Solvable())
    assert isinstance(builder._constraint.predicate, Solvable)


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
    assert n_lasers >= 2, f"Expected auto n_lasers >= 2 for chained predicate with n_agents=2, got {n_lasers}"


def test_default_lasers_mutual_n_agents_2():
    """Same default-value fix applies to the mutual filter (mentioned in the todo).

    generate(width=5, height=5, n_agents=2).mutual().build() would also have
    raised without the fix because mutual implies chained cooperation.
    """
    builder = generate(width=5, height=5, n_agents=2).mutual()
    placement = builder._resolve_placement(builder._starts)
    n_lasers = builder._resolve_n_lasers(placement)
    assert n_lasers >= 2, f"Expected auto n_lasers >= 2 for mutual predicate with n_agents=2, got {n_lasers}"


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
