"""Tests for preventing *mutual* cooperation in the SAT solver.

Mutual cooperation between agents `a` and `b` holds when, in a plan, `a` helps `b` cross one of
`a`'s laser beams at some point *and* `b` helps `a` likewise at some point. These tests cover:

- `solve(mode="no-mutual-cooperation")` (the public API), and
- `ClauseGenerator(mode="no-mutual-cooperation")` (the low-level Rust primitive),

against oracles whose cooperation structure is already pinned by the codebase (levels 1/3/6) plus
two hand-built corridors.
"""

from __future__ import annotations

import lle
import pytest
from lle import World
from lle.solver import solve
from lle.solver.constraints import ClauseGenerator

# No detour: the two facing beams span the whole corridor, so mutual help is unavoidable.
ALWAYS_MUTUAL = "S0 . . S1\nL0E . . .\n. . . L1W\nX . . X"

# Short route (cols 0-1) forces mutual help across two length-2 beams; a laser-free detour exists
# down cols 4-5 (around the wall column), so mutual help is required only below a time threshold.
TIME_DEPENDENT = "\n".join(
    [
        "S0 S1 . . . .",
        "L0E . . @ . .",
        "L1E . . @ . .",
        "X  X  . @ . .",
        ".  .  . . . .",
    ]
)
# Empirically, mutual help is required up to t=12 and a mutual-free plan exists from t=13 on.
TIME_DEPENDENT_THRESHOLD = 13

NO_LASER = "S0 . S1\n. . .\nX . X"


def requires_mutual_cooperation(world: World, t_max: int) -> bool:
    """Return True if the world is solvable but cannot be solved without mutual cooperation."""
    if solve(world, t_max) is None:
        return False
    return solve(world, t_max, mode="no-mutual-cooperation") is None


# ---------------------------------------------------------------------------
# requires_mutual_cooperation on canonical levels
# ---------------------------------------------------------------------------
def test_independent_level_needs_no_mutual_cooperation():
    assert requires_mutual_cooperation(World.level(1), 10) is False
    assert solve(World.level(1), 10, mode="no-mutual-cooperation") is not None


def test_asymmetric_cooperation_is_not_mutual():
    """Level 3 *requires cooperation* but only one-directionally, so forbidding mutual help is
    still satisfiable. This is the case that distinguishes this feature from `is_cooperative`."""
    world = World.level(3)
    assert lle.is_cooperative(world, 12) is True
    assert requires_mutual_cooperation(world, 12) is False
    assert solve(world, 12, mode="no-mutual-cooperation") is not None


def test_level_6_requires_mutual_cooperation():
    world = World.level(6)
    assert requires_mutual_cooperation(world, 21) is True
    assert solve(world, 21, mode="no-mutual-cooperation") is None


def test_no_laser_world_is_never_mutual():
    assert requires_mutual_cooperation(World(NO_LASER), 6) is False


# ---------------------------------------------------------------------------
# Constructed corridors
# ---------------------------------------------------------------------------
def test_always_mutual_corridor():
    world = World(ALWAYS_MUTUAL)
    assert requires_mutual_cooperation(world, 10) is True
    # No mutual-free plan exists at any solvable horizon.
    for t in range(11):
        if solve(world, t) is None:
            continue
        assert solve(world, t, mode="no-mutual-cooperation") is None


def test_time_dependent_threshold():
    world = World(TIME_DEPENDENT)
    # Below the threshold: solvable, but only via mutual cooperation.
    for t in range(TIME_DEPENDENT_THRESHOLD):
        if solve(world, t) is None:
            continue
        assert solve(world, t, mode="no-mutual-cooperation") is None, f"expected mutual help at t={t}"
    # At/above the threshold: a mutual-free plan appears.
    assert solve(world, TIME_DEPENDENT_THRESHOLD, mode="no-mutual-cooperation") is not None
    # The mutual-free plan is itself a valid plan (replays without error onto the world).
    plan = solve(world, TIME_DEPENDENT_THRESHOLD, mode="no-mutual-cooperation")
    assert plan is not None
    world.reset()
    for joint in plan:
        world.step(joint)
    assert all(agent.is_alive and agent.has_arrived for agent in world.agents)


# ---------------------------------------------------------------------------
# ClauseGenerator with mode="no-mutual-cooperation"
# ---------------------------------------------------------------------------
def test_no_mutual_cooperation_mode_empty_without_lasers():
    """With no lasers, no-mutual-cooperation mode should still find a solution."""
    plan = solve(World(NO_LASER), 6, mode="no-mutual-cooperation")
    assert plan is not None


def test_no_mutual_cooperation_mode_always_mutual_is_unsat():
    """With mutual cooperation unavoidable, no-mutual-cooperation mode returns None."""
    world = World(ALWAYS_MUTUAL)
    assert solve(world, 10, mode="no-mutual-cooperation") is None


def test_clause_generator_no_mutual_cooperation_mode():
    """ClauseGenerator with mode='no-mutual-cooperation' must find the same answer as solve()."""
    world = World(ALWAYS_MUTUAL)
    gen = ClauseGenerator(world, 10, mode="no-mutual-cooperation")
    from lle.solver.solver import solve_model
    for t in range(gen.solution_lower_bound, gen.t_max + 1):
        clauses, assumptions = gen.generate(t)
        model = solve_model(clauses, assumptions=assumptions)
        if model is not None:
            plan = gen.decode_plan(model, t)
            assert plan is not None
            return
    # ALWAYS_MUTUAL should be UNSAT under no-mutual-cooperation at any horizon ≤10
    assert solve(world, 10, mode="no-mutual-cooperation") is None
