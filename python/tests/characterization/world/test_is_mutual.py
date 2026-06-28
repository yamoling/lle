import pytest
from lle import World
from lle.characterization import WorldCharacterizer


def test_is_obviously_mutual():
    world = World("""
     S0 . . S1
    L0E . . .
     .  . . L1W
     X  . . X""")
    wc = WorldCharacterizer(world, 6)
    assert wc.is_mutual()


def test_unsolvable_world_raises_on_is_mutual():
    world = World("S0 @ X")
    with pytest.raises(ValueError):
        _ = WorldCharacterizer(world, t_max=10).is_mutual()


def test_two_agent_mutual_is_chain_2_but_not_chain_3():
    """A two-agent cycle is the smallest chain/interdependence case and cannot satisfy length 3."""
    wc = WorldCharacterizer(
        World("""
     .  . . S0 S1  .  . . .
    L0E . .  .  .  @  @ @ .
     .  . @  .  . L1W . . .
     .  . .  .  .  .  . . .
     .  . .  X  X  .  . . .
    """),
        t_max=6,
    )
    assert wc.is_solvable()
    assert wc.is_mutual()
    assert wc.is_chained(2)
    assert not wc.is_chained(3)
    assert wc.is_interdependent(2)
    assert not wc.is_interdependent(3)


def test_level6_requires_mutual_cooperation():
    """Level 6: mutual cooperation required."""
    wc = WorldCharacterizer(World.level(6), t_max=21)
    assert wc.is_solvable()
    assert wc.is_cooperative()
    assert wc.is_mutual()
    assert wc.is_chained(2)
    for i in range(3, 10):
        assert not wc.is_chained(i)


def test_threshold_mutual_to_cooperative():
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
    for t_max in range(5, 15):
        wc = WorldCharacterizer(world, t_max)
        is_cooperative = t_max < 12
        is_mutual = t_max < 8
        assert wc.is_solvable()
        assert wc.is_independent() != is_cooperative
        assert wc.is_cooperative() == is_cooperative
        assert wc.is_mutual() == is_mutual
