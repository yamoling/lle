from lle import World
from lle.characterization import WorldCharacterizer


def test_paper_example_distributed2():
    world = World("""
        @   S0  .  S2  .
        L0E .   .  .   @
        @   X   @  .   .
        @   L1E .  S1  .
        @   @   @  X   X""")
    wc = WorldCharacterizer(world, t_max=10)
    assert wc.is_solvable()
    assert wc.is_distributed(2)
    assert not wc.is_distributed(3)
    assert not wc.is_chained(2)
    assert not wc.is_interdependent(2)
    assert not wc.is_asymmetric()
    assert not wc.is_chained(2)
