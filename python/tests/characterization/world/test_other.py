from lle import World
from lle.characterization import WorldCharacterizer


def test_eq_and_hash():
    world = World.level(1)
    a = WorldCharacterizer(world, t_max=10)
    b = WorldCharacterizer(world, t_max=10)
    different_t_max = WorldCharacterizer(world, t_max=11)
    assert a == b
    assert hash(a) == hash(b)
    assert a != different_t_max
    assert a != "not a characterizer"


def test_is_interdependent_is_cached():
    """A second query with the same order returns the cached result."""
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
    first = wc.is_interdependent(2)
    second = wc.is_interdependent(2)
    assert first is second
    assert first


def test_unsolvable_world_is_not_solvable():
    world = World("S0 @ X")
    assert not WorldCharacterizer(world, t_max=10).is_solvable()
