"""Tests for predicate constraints and constrained world generation.

Section A: Constraint.is_satisfied_by — fast SAT checks against known levels.
Section B: the generation builder with predicate shortcuts — integration tests.
"""

from __future__ import annotations

import pytest
from lle import World
from lle.generator import Chained, Constraint, Cooperative, Independent, Interdependent, Solvable, WorldRequirements, generate


@pytest.mark.parametrize(("level", "solution_length"), [(1, 10), (2, 10), (3, 10), (4, 10), (5, 19), (6, 21)])
def test_standard_level_t_min(level: int, solution_length):
    world = World.level(level)
    c = Constraint(solution_length)
    assert c.is_satisfied_by(world)


@pytest.mark.parametrize(("level", "solution_length"), [(1, 10), (2, 10), (3, 10), (4, 10), (5, 19), (6, 21)])
def test_standard_level_with_high_t_min(level: int, solution_length):
    """
    We build a constraint whose t_min requirement is above the actual solution limit.
    Therefore, the world should be rejected since a solution exists with a length <= `t_min`."""
    world = World.level(level)
    c = Constraint(solution_length + 5, t_min=solution_length + 1)
    assert not c.is_satisfied_by(world)


def test_filter_no_constraints_accepts_solvable_world():
    world = World("S0 . S1\n.  .  .\nX  .  X")
    assert Constraint(10).is_satisfied_by(world)


def test_filter_no_constraints_rejects_unsolvable_world():
    world = World("S0 @ X")
    assert not Constraint(10).is_satisfied_by(world)


def test_filter_cooperative_standard_levels():
    """Level 6 requires cooperation — Cooperative should accept it."""
    constraint_cooperative = Constraint(21, Cooperative())
    constraint_independent = Constraint(21, Independent())
    for level in [1, 2]:
        world = World.level(level)
        assert not constraint_cooperative.is_satisfied_by(world)
        assert constraint_independent.is_satisfied_by(world)
    for level in [3, 4, 5, 6]:
        world = World.level(level)
        assert constraint_cooperative.is_satisfied_by(world)
        assert not constraint_independent.is_satisfied_by(world)


def test_interdependent_level6():
    """Level 6 is 2-interdependent, not 3-interdependent."""
    inter2 = Constraint(21, Interdependent(2))
    inter3 = Constraint(21, Interdependent(3))
    world = World.level(6)
    assert inter2.is_satisfied_by(world)
    assert not inter3.is_satisfied_by(world)


def test_mutual_standard_levels():
    mutual = Constraint(21, Interdependent(2))
    for level in [1, 2, 3, 5]:
        world = World.level(level)
        assert not mutual.is_satisfied_by(world)
    for level in [4, 6]:
        world = World.level(level)
        assert mutual.is_satisfied_by(world)


def test_constraint_uses_its_t_max():
    constraint = Constraint(21, Cooperative())
    # t_max=21 → level 6 is solvable and cooperative → True
    assert constraint.is_satisfied_by(World.level(6))
    # t_max=20 → level 6 is unsolvable → False
    constraint = Constraint(20, Cooperative())
    assert not constraint.is_satisfied_by(World.level(6))


def test_filter_mutual_rejects_level6_with_insufficient_t_max():
    """Level 6 needs exactly 21 steps; t_max=20 makes it unsolvable → Mutual fails."""
    assert not Constraint(20, Interdependent(2)).is_satisfied_by(World.level(6))


def test_world_requirements_for_atoms():
    assert Solvable().requirements == WorldRequirements()
    assert Independent().requirements == WorldRequirements()
    assert Cooperative().requirements == WorldRequirements(min_lasers=1, min_agents=2)
    assert Interdependent(2).requirements == WorldRequirements(min_lasers=2, min_agents=2)


def test_world_requirements_compose_over_boolean_expressions():
    assert (Chained(3) & ~Interdependent(2)).requirements == WorldRequirements(min_lasers=3, min_agents=2)
    assert (Interdependent(2) | Interdependent(5)).requirements == WorldRequirements(min_lasers=2, min_agents=2)
    assert (~Interdependent(5)).requirements == WorldRequirements()


def test_generate_cooperative_shortcut_produces_cooperative_world():
    world = generate(width=5, height=5, n_agents=2).lasers(2).cooperative().build()
    assert Constraint(20, Cooperative()).is_satisfied_by(world)


def test_generate_independent_produces_independent_world():
    t_max = 40
    world = generate(t_max=t_max).independent().build()
    assert Constraint(t_max, Independent()).is_satisfied_by(world)


def test_last_filter_call_wins():
    builder = generate(width=5, height=5, n_agents=2).cooperative().interdependent(2)
    assert isinstance(builder._constraint.predicate, Interdependent)
    builder = generate(width=5, height=5, n_agents=2).interdependent(2).independent()
    assert isinstance(builder._constraint.predicate, Independent)


def test_require_overrides_named_filter():
    builder = generate(width=5, height=5, n_agents=2).cooperative().require(Solvable())
    assert isinstance(builder._constraint.predicate, Solvable)


def test_generate_error_cooperative_n_agents_lt_2():
    with pytest.raises(ValueError, match="agents"):
        generate(width=5, height=5, n_agents=1).cooperative().build(max_attempts=1)


def test_generate_error_cooperative_n_lasers_0():
    with pytest.raises(ValueError, match="laser"):
        generate(width=5, height=5, n_agents=2).lasers(0).cooperative().build(max_attempts=1)


def test_generate_error_mutual_n_agents_lt_2():
    with pytest.raises(ValueError, match="agents"):
        generate(width=5, height=5, n_agents=1).interdependent(2).build(max_attempts=1)


def test_generate_error_mutual_n_lasers_lt_2():
    with pytest.raises(ValueError, match="laser"):
        generate(width=5, height=5, n_agents=2).lasers(1).interdependent(2).build(max_attempts=1)


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

    generate(width=5, height=5, n_agents=2).Interdependent(2).build() would also have
    raised without the fix because mutual implies chained cooperation.
    """
    builder = generate(width=5, height=5, n_agents=2).interdependent(2)
    placement = builder._resolve_placement(builder._starts)
    n_lasers = builder._resolve_n_lasers(placement)
    assert n_lasers >= 2, f"Expected auto n_lasers >= 2 for mutual predicate with n_agents=2, got {n_lasers}"


def test_default_lasers_chained_does_not_raise_on_build():
    """End-to-end: generate().lanes().chained().build() must not raise ValueError."""
    generate(width=5, height=5, n_agents=2).lanes().chained().build(max_attempts=1)


def test_default_lasers_mutual_does_not_raise_on_build():
    """End-to-end: generate().Interdependent(2).build() must not raise ValueError."""
    generate(width=5, height=5, n_agents=2).interdependent(2).build(max_attempts=1)


def test_explicit_laser_count_overrides_default():
    """An explicit lasers() call must always win over the smart default."""
    builder = generate(width=5, height=5, n_agents=3).lasers(3).chained()
    placement = builder._resolve_placement(builder._starts)
    n_lasers = builder._resolve_n_lasers(placement)
    assert n_lasers == 3
