"""Tests that the t_min constraint rejects worlds solvable in fewer than
t_min steps.

Section A: unit tests for Generator._accept_world against known worlds.
Section B: integration tests for the builder's within(t_min=...) constraint.
"""

from __future__ import annotations

import pytest
from lle import World
from lle.generator import generate
from lle.generator.generator import WorldGenerator
from lle.solver import solve

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------


def _gen(t_min: int, t_max: int = 20):
    """Minimal single-agent generator for testing _accept_world."""
    from lle.generator.world_filter import Solvable

    return WorldGenerator(height=5, width=5, n_agents=1, n_walls=0, n_lasers=0, filter=Solvable(t_max=t_max, t_min=t_min))


# ---------------------------------------------------------------------------
# A. Unit tests — _accept_world against known worlds
# ---------------------------------------------------------------------------


def test_accept_world_rejects_world_solvable_below_t_min():
    """A world solvable in 2 steps must be rejected when t_min=4."""
    # S0 . X → agent at (0,0), exit at (0,2); shortest solution is 2 East steps
    world = World("S0 . X")
    assert solve(world, 2) is not None, "sanity: world is solvable in 2 steps"
    assert solve(world, 1) is None, "sanity: not solvable in 1 step"

    gen = _gen(t_min=4)
    assert not gen._accept_world(world)


def test_accept_world_t_min_zero_no_lower_bound():
    """t_min=0 (the default) must not add any lower-bound constraint."""
    world = World("S0 . X")  # solvable in 2 steps
    gen = _gen(t_min=0)
    assert gen._accept_world(world)


def test_accept_world_accepts_world_solvable_exactly_at_t_min():
    """A world whose shortest solution is exactly t_min steps must be accepted."""
    # S0 . . . X → exit at col 4; shortest solution is 4 East steps
    world = World("S0 . . . X")
    assert solve(world, 4) is not None, "sanity: solvable in 4 steps"
    assert solve(world, 3) is None, "sanity: not solvable in 3 steps"

    gen = _gen(t_min=4)
    assert gen._accept_world(world)


def test_accept_world_rejects_world_solvable_one_step_short():
    """Edge case: world solvable in exactly t_min-1 steps must be rejected."""
    # S0 . . X → exit at col 3; shortest solution is 3 steps
    world = World("S0 . . X")
    t_min = 4
    assert solve(world, t_min - 1) is not None, "sanity: solvable in t_min-1 steps"
    assert solve(world, t_min - 2) is None, "sanity: not solvable in t_min-2 steps"

    gen = _gen(t_min=t_min)
    assert not gen._accept_world(world)


def test_accept_world_accepts_t_min_one_for_immediately_solvable_world():
    """t_min=1 should accept a world that requires at least 1 step."""
    # S0 . X → requires 2 steps → OK for t_min=1
    world = World("S0 . X")
    gen = _gen(t_min=1)
    assert gen._accept_world(world)


# ---------------------------------------------------------------------------
# B. Integration tests — the builder's within(t_min=...) constraint
# ---------------------------------------------------------------------------


def test_generate_t_min_world_not_solvable_below_t_min():
    """within(t_min=...) must return a world not solvable in fewer than t_min steps."""
    t_min = 4
    world = generate(width=5, height=5, n_agents=2).lanes().at_least(t_min).build(max_attempts=200)
    assert world is not None
    assert solve(world, t_min - 1) is None, "world is solvable in fewer than t_min steps"


def test_generate_t_min_world_is_solvable_within_t_max():
    """within(t_min, t_max) must still return a world solvable within t_max."""
    t_min, t_max = 3, 15
    world = generate(width=5, height=5, n_agents=2).random().at_least(t_min).cap(t_max).build(seed=0, max_attempts=2000)
    assert world is not None
    assert solve(world, t_max) is not None, "world must be solvable within t_max"


def test_generate_multiple_worlds_all_respect_t_min():
    """Every world in a batch must satisfy the t_min constraint."""
    t_min, t_max = 3, 15
    worlds = list(
        generate(width=5, height=5, n_agents=2).random().at_least(t_min).cap(t_max).take(5, n_jobs=1, max_attempts=2000, progress=False)
    )
    assert len(worlds) > 0, "generator produced no worlds within the attempt budget"
    for world in worlds:
        assert solve(world, t_min - 1) is None, "a world in the batch was solvable in fewer than t_min steps"
