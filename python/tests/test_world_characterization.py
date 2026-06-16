"""Tests for WorldCharacterizer.

Verifies `is_independent`, `is_cooperative`, and `is_mutual` against the six
standard levels and a hand-crafted world whose cooperation threshold is known
exactly.
"""

from __future__ import annotations

import pytest
from lle import World
from lle.characterization.world_characterization import WorldCharacterizer

ONE_WAY_COOPERATION = """
 .  . S0 S1 . .
L0E .  .  . @ .
 .  .  .  . . .
 .  .  .  . . .
 X  X  .  . . .
"""


TWO_AGENT_MUTUAL = """
 .  . . S0 S1  .  . . .
L0E . .  .  .  @  @ @ .
 .  . @  .  . L1W . . .
 .  . .  .  .  .  . . .
 .  . .  X  X  .  . . .
"""


TWO_AGENT_INTERDEPENDENT_ONLY = """
S0  .  .  .  .  @ @
S1  .  .  .  .  @ @
S2 L0E .  .  .  @ @
.  .  .  @  .  . .
.  .  .  . L1W . .
.  @ L2E .  .  . .
.  @  @  .  X  X X
"""


THREE_AGENT_INTERDEPENDENT = """
 @ L0S L2S L1S .
S0  .   .   .  X
 .  .   .   .  .
S1  .   .   .  X
S2  .   .   .  X
"""


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
    world = World(ONE_WAY_COOPERATION)
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
    world = World(TWO_AGENT_MUTUAL)
    wc = WorldCharacterizer(world, t_max)
    is_cooperative = t_max < 12
    is_mutual = t_max < 8
    assert wc.is_solvable
    assert wc.is_independent != is_cooperative
    assert wc.is_cooperative == is_cooperative
    assert wc.is_mutual == is_mutual


def test_one_way_cooperation_is_not_chained_or_interdependent():
    """A single required help edge is cooperative, but not a temporal chain or cycle."""
    wc = WorldCharacterizer(World(ONE_WAY_COOPERATION), t_max=8)
    assert wc.is_solvable
    assert wc.is_cooperative
    assert not wc.is_mutual
    assert not wc.is_chained(2)
    assert not wc.is_chained(3)
    assert not wc.is_interdependent(2)
    assert not wc.is_interdependent(3)


def test_two_agent_mutual_is_chain_2_but_not_chain_3():
    """A two-agent cycle is the smallest chain/interdependence case and cannot satisfy length 3."""
    wc = WorldCharacterizer(World(TWO_AGENT_MUTUAL), t_max=6)
    assert wc.is_solvable
    assert wc.is_mutual
    assert wc.is_chained(2)
    assert not wc.is_chained(3)
    assert wc.is_interdependent(2)
    assert not wc.is_interdependent(3)


def test_two_agent_cycle_in_three_agent_world_is_not_3_interdependent():
    """The world requires a mutual cycle, but the third agent can avoid joining that cycle."""
    wc = WorldCharacterizer(World(TWO_AGENT_INTERDEPENDENT_ONLY), t_max=16)
    assert wc.is_solvable
    assert wc.is_chained(2)
    assert not wc.is_chained(3)
    assert wc.is_interdependent(2)
    assert not wc.is_interdependent(3)


def test_three_agent_cycle_is_3_interdependent_but_not_4_interdependent():
    """A 3-agent cycle exercises the parametrized chain/interdependence upper edge."""
    wc = WorldCharacterizer(World(THREE_AGENT_INTERDEPENDENT), t_max=15)
    assert wc.is_solvable
    assert wc.is_chained(2)
    assert wc.is_chained(3)
    assert not wc.is_chained(4)
    assert wc.is_interdependent(2)
    assert wc.is_interdependent(3)
    assert not wc.is_interdependent(4)


def test_chain_4_is_not_interdependent():
    """A 4-agent chain is not an interdependent."""
    world = World("""
 @  S0 S1  @
L0E X  .   @
 @  S2 .   @
 @  .  X  L1W
 @  .  S3  @
L2E X  .   @
 @  .  X   @
""")
    wc = WorldCharacterizer(world, t_max=6)
    assert wc.is_solvable
    assert wc.is_chained(2)
    assert wc.is_chained(3)
    assert not wc.is_chained(4)
    assert not wc.is_mutual
    assert not wc.is_interdependent(2)
    assert not wc.is_interdependent(3)
    assert not wc.is_interdependent(4)
    assert wc.shortest_non_interdependent_path(2) is not None


def test_chain_4_and_mutual():
    """A 4-agent chain is not an interdependent."""
    world = World("""
 @  S0 S1  @
L0E X  .   @
 @  S2 .   @
 @  .  X  L1W
 @  .  S3  @
L2E .  .   @
 @  X  X  L3W
""")
    wc = WorldCharacterizer(world, t_max=6)
    assert wc.is_solvable
    assert wc.is_chained(2)
    assert wc.is_chained(3)
    assert wc.is_chained(4)
    assert wc.is_mutual
    assert wc.is_interdependent(2)
    assert not wc.is_interdependent(3)
    assert not wc.is_interdependent(4)
    assert wc.shortest_non_interdependent_path(2) is None
    assert wc.shortest_non_interdependent_path(3) is not None


def test_no_3cycle_because_of_temporality():
    """
    We want to show that temporality is important and that a temporally-flattened graph
    cannot represent the actual cooperation graph.

    In this world, we have a first step where two independent help events occur:
        - help(0, 1, t=1)
        - help(2, 3, t=1)
    Then, we have
        - help(1, 2, t=2)
        - help(3, 1, t=2)
    In a flattened graph, we would have a cycle 0 -> 1 -> 2 -> 3 -> 0 while there is actually
    no dependency between 0 and 3.

    To do so, the map is organized is such that:
       - the two bottom exits are only available toagents 0 and 2 because they stand in a laser beam;
       - agents 1 and 3 have no choice but to walk on an exit tile right away
       - agents 1 and 3 block a laser from their exit tiles.
    """
    world = World("""
 @  @  L1S @ L3S  @   @
 @  S0 S1  @ S3  S2   @
L0E .   .  @  .   .  L2W
 @  .   X  @  X   .   @
 @  .   .  .  .   .   @
 @ L2E  X  @  X  L0W  @
""")
    wc = WorldCharacterizer(world, 20)
    assert wc.is_cooperative
    assert not wc.is_independent
    assert wc.is_mutual  # 0 helps 1 and vice-versa
    assert wc.is_chained(2)  # Equivalent to is_mutual
    assert not wc.is_chained(3)
    assert wc.is_interdependent(2)  # Equivalent to is_mutual
    assert not wc.is_interdependent(3)


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
