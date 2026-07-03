import lle
import pytest
from lle import World
from lle.characterization import WorldCharacterizer


def test_simple_solvable():
    world = World("""
 . . . . X
S0 . . . .
S1 . . . .
 . . . . X
""")
    path = lle.solve(world)
    assert path is not None
    assert not lle.is_cooperative(world)


def test_standard_levels_cooperation():
    for level, t_max in zip((1, 2, 3, 4, 5, 6), (10, 10, 10, 10, 19, 21)):
        world = World.level(level)
        wc = WorldCharacterizer(world, t_max)
        cooperation_expected = level >= 3
        assert wc.is_cooperative() == cooperation_expected
        assert wc.is_independent() == (not cooperation_expected)


def test_unsolvable_world_raises_on_is_cooperative():
    world = World("S0 @ X")
    with pytest.raises(ValueError):
        _ = WorldCharacterizer(world, t_max=10).is_cooperative()


def test_threshold_is_independent():
    """At t=10 the long detour becomes reachable: cooperation is no longer required."""
    # For t < 10: every solution forces agent 0 to block its own laser for agent 1.
    # For t> = 10: agent 1 can go around via column 5, so no blocking is required.
    world = World("""
     .  . S0 S1 . .
    L0E .  .  . @ .
     .  .  .  . . .
     .  .  .  . . .
     X  X  .  . . .
    """)
    for t_max, is_cooperative in [(8, True), (9, True), (10, False), (11, False)]:
        wc = WorldCharacterizer(world, t_max)
        assert wc.is_solvable()
        assert wc.is_independent() != is_cooperative
        assert wc.is_cooperative() == is_cooperative
        assert not wc.is_interdependent(2)
