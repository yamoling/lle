import pytest
from lle import World
from lle.characterization import WorldCharacterizer


def test_is_chained_rejects_length_below_2():
    wc = WorldCharacterizer(World.level(1), t_max=10)
    for length in [1, 0, -1]:
        with pytest.raises(ValueError):
            wc.is_chained(length)


def test_unsolvable_world_raises_on_is_chained():
    world = World("S0 @ X")
    with pytest.raises(ValueError):
        _ = WorldCharacterizer(world, t_max=10).is_chained(2)


def test_chain_4_and_mutual():
    """A 4-agent chain with 2-interdependence between agents 2 and 3."""
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
    assert wc.is_solvable()
    assert wc.is_chained(2)
    assert wc.is_chained(3)
    assert wc.is_chained(4)
    assert wc.is_interdependent(2)
    assert not wc.is_interdependent(3)
    assert not wc.is_interdependent(4)
    assert wc.compute_shortest_non_interdependent_path(2) is None
    assert wc.compute_shortest_non_interdependent_path(3) is not None


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
    assert wc.is_solvable()
    assert wc.is_chained(2)
    assert wc.is_chained(3)
    assert not wc.is_chained(4)
    assert not wc.is_interdependent(2)
    assert not wc.is_interdependent(3)
    assert not wc.is_interdependent(4)
    assert wc.compute_shortest_non_interdependent_path(2) is not None


def test_paper_example_c2():
    world = World("""
    @  S0 @ S1 @
   L0E .  . .  @
    @  X  @ . S2
    @ L1E . .  .
    @  @  @ X  x""")
    wc = WorldCharacterizer(world, t_max=10)
    assert wc.is_solvable()
    assert wc.is_chained(2)
    assert not wc.is_chained(3)
    assert not wc.is_distributed(2)
    assert not wc.is_chained(3)
    assert not wc.is_interdependent(2)
