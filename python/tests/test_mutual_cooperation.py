"""Tests for preventing *mutual* cooperation in the SAT solver.

Mutual cooperation between agents `a` and `b` holds when, in a plan, `a` helps `b` cross one of
`a`'s laser beams at some point *and* `b` helps `a` likewise at some point. These tests cover:

- `ClauseGenerator.dependency_clauses` / `forbid_mutual_cooperation` (the Rust primitives), and
- `solve_without_mutual_cooperation` / `requires_mutual_cooperation` (the Python helpers),

against oracles whose cooperation structure is already pinned by the codebase (levels 1/3/6) plus
two hand-built corridors.
"""

from __future__ import annotations

import lle
import pytest
from lle import World
from lle.solver import requires_mutual_cooperation, solve, solve_without_mutual_cooperation
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


# ---------------------------------------------------------------------------
# requires_mutual_cooperation on canonical levels
# ---------------------------------------------------------------------------
def test_independent_level_needs_no_mutual_cooperation():
    assert requires_mutual_cooperation(World.level(1), 10) is False
    assert solve_without_mutual_cooperation(World.level(1), 10) is not None


def test_asymmetric_cooperation_is_not_mutual():
    """Level 3 *requires cooperation* but only one-directionally, so forbidding mutual help is
    still satisfiable. This is the case that distinguishes this feature from `is_cooperative`."""
    world = World.level(3)
    assert lle.is_cooperative(world, 12) is True
    assert requires_mutual_cooperation(world, 12) is False
    assert solve_without_mutual_cooperation(world, 12) is not None


def test_level_6_requires_mutual_cooperation():
    world = World.level(6)
    assert requires_mutual_cooperation(world, 21) is True
    assert solve_without_mutual_cooperation(world, 21) is None


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
        assert solve_without_mutual_cooperation(world, t) is None


def test_time_dependent_threshold():
    world = World(TIME_DEPENDENT)
    # Below the threshold: solvable, but only via mutual cooperation.
    for t in range(TIME_DEPENDENT_THRESHOLD):
        if solve(world, t) is None:
            continue
        assert solve_without_mutual_cooperation(world, t) is None, f"expected mutual help at t={t}"
    # At/above the threshold: a mutual-free plan appears.
    assert solve_without_mutual_cooperation(world, TIME_DEPENDENT_THRESHOLD) is not None
    # The mutual-free plan is itself a valid plan (replays without error onto the world).
    plan = solve_without_mutual_cooperation(world, TIME_DEPENDENT_THRESHOLD)
    assert plan is not None
    world.reset()
    for joint in plan:
        world.step(joint)
    assert all(agent.is_alive and agent.has_arrived for agent in world.agents)


# ---------------------------------------------------------------------------
# Clause-generator primitives (bindings)
# ---------------------------------------------------------------------------
def _generate_all(world: World, t_max: int) -> ClauseGenerator:
    gen = ClauseGenerator(world, t_max)
    for t in range(t_max + 1):
        gen.generate(t)
        gen.dependency_clauses(t)
    return gen


def test_forbid_mutual_cooperation_is_empty_without_lasers():
    gen = _generate_all(World(NO_LASER), 6)
    clauses, assumptions = gen.forbid_mutual_cooperation()
    assert clauses == [] and assumptions == []


def test_forbid_mutual_cooperation_reifies_the_pair():
    gen = _generate_all(World(ALWAYS_MUTUAL), 8)
    clauses, assumptions = gen.forbid_mutual_cooperation()
    # A single agent pair {0, 1}, both directions expressible.
    assert len(clauses) == 1
    mutual = gen.mutual_lit(0, 1)
    assert mutual is not None
    # The assumption forbids that mutual variable, and the clause is its reifying definition.
    assert assumptions == [-mutual]
    assert mutual in clauses[0]
    assert len(clauses[0]) == 3  # depends_on(1,0) ∧ depends_on(0,1) → mutual(0,1)


def test_dependency_clauses_are_binary_implications():
    gen = ClauseGenerator(World(ALWAYS_MUTUAL), 8)
    for t in range(9):
        gen.generate(t)
        for clause in gen.dependency_clauses(t):
            assert len(clause) == 2
            # exactly one negated (agent) and one positive (depends_on) literal
            assert sum(1 for lit in clause if lit < 0) == 1
            assert sum(1 for lit in clause if lit > 0) == 1
