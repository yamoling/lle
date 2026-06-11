"""Tests for WorldCharacterizer.

Verifies `is_independent`, `is_cooperative`, and `is_mutual` against the six
standard levels and a hand-crafted world whose cooperation threshold is known
exactly.  All tests invoke the SAT solver, so they may be slow; keep a 60-second
budget in mind per CLAUDE.md.
"""

from __future__ import annotations

import pytest
from lle import World
from lle.characterization.world_characterization import WorldCharacterizer


# ---------------------------------------------------------------------------
# World with no lasers: trivially independent
# ---------------------------------------------------------------------------
def test_no_laser_world_is_independent():
    """Without any laser sources no blocking can occur: always independent."""
    world = World("S0 . S1\n.  .  .\nX  .  X")
    wc = WorldCharacterizer(world, t_max=6)
    assert wc.is_solvable is True
    assert wc.is_independent is True
    assert wc.is_mutual is False


@pytest.mark.parametrize("level", [1, 2])
def test_level1_and_2_are_independent(level: int):
    """Level 1 needs no laser blocking: independently solvable."""
    wc = WorldCharacterizer(World.level(level), t_max=10)
    assert wc.is_solvable
    assert wc.is_independent
    assert not wc.is_cooperative


@pytest.mark.parametrize("level", [3, 4, 5])
def test_cooperative_levels_require_cooperation(level: int):
    """Levels 3-5: cooperation required but not mutual"""
    wc = WorldCharacterizer(World.level(level), t_max=21)
    assert wc.is_solvable
    assert wc.is_cooperative
    assert not wc.is_independent


def test_level6_requires_mutual_cooperation():
    """Level 6: mutual cooperation required."""
    wc = WorldCharacterizer(World.level(6), t_max=21)
    assert wc.is_solvable is True
    assert wc.is_cooperative is True
    assert wc.is_mutual is True


@pytest.mark.parametrize(("t_max", "is_cooperative"), [(8, True), (9, True), (10, False), (11, False)])
def test_poc_threshold_is_independent(t_max: int, is_cooperative: bool):
    """At t=10 the long detour becomes reachable: cooperation is no longer forced."""
    # For t < 10: every solution forces agent 0 to block its own laser for agent 1.
    # For t> = 10: agent 1 can go around via column 5, so no blocking is required.
    world = World("""
     .  . S0 S1 . .
    L0E .  .  . @ .
     .  .  .  . . .
     .  .  .  . . .
     X  X  .  . . .
""")
    wc = WorldCharacterizer(world, t_max)
    assert wc.is_solvable
    assert wc.is_independent != is_cooperative
    assert wc.is_cooperative == is_cooperative
    assert not wc.is_mutual


@pytest.mark.parametrize("t_max", range(5, 15))
def test_threshold_mutual_to_cooperative(t_max: int):
    """
    The world is designed such that:
        - < 8 steps, mutual help is required
        - 8 <= steps < 12, mutual help is no longer required because agent 0 can
        take a detour behind the left wall that blocks beam 1; but the level remains cooperative.
        - >= 12 steps, the level is independent
    """
    world = World("""
     .  . . S0 S1  .  . . .
    L0E . .  .  .  @  @ @ .
     .  . @  .  . L1W . . .
     .  . .  .  .  .  . . .
     .  . .  X  X  .  . . .
""")
    wc = WorldCharacterizer(world, t_max)
    is_cooperative = t_max < 12
    is_mutual = t_max < 8
    assert wc.is_solvable
    assert wc.is_independent != is_cooperative
    assert wc.is_cooperative == is_cooperative
    assert wc.is_mutual == is_mutual


# ---------------------------------------------------------------------------
# Unsolvable world: error handling
# ---------------------------------------------------------------------------
def test_unsolvable_world_is_not_solvable():
    world = World("S0 @ X")
    assert not WorldCharacterizer(world, t_max=10).is_solvable


def test_unsolvable_world_raises_on_is_cooperative():
    world = World("S0 @ X")
    with pytest.raises(ValueError):
        _ = WorldCharacterizer(world, t_max=10).is_cooperative


def test_unsolvable_world_raises_on_is_independent():
    world = World("S0 @ X")
    with pytest.raises(ValueError):
        _ = WorldCharacterizer(world, t_max=10).is_independent


def test_unsolvable_world_raises_on_is_mutual():
    world = World("S0 @ X")
    with pytest.raises(ValueError):
        _ = WorldCharacterizer(world, t_max=10).is_mutual
