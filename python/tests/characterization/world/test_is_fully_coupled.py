from lle import World
from lle.characterization import WorldCharacterizer


def assert_fully_coupled(wc: WorldCharacterizer):
    """Assertion for canonical fully coupled world"""
    assert wc.is_solvable()
    for n in range(2, wc.world.n_agents + 1):
        assert wc.is_interdependent(n)
    assert wc.is_fully_coupled()
    assert not wc.is_asymmetric()
    assert not wc.is_independent()
    assert wc.is_cooperative()


def test_paper_example_valid():
    world = World("""
        @  L0S  @ @ @ @
        S0  .   . . @ @
        S1  .   . . . @
        S2  .   . . . @
        @   L2E . . . @
        @   @   X X X L1W""")
    wc = WorldCharacterizer(world, t_max=10)
    assert_fully_coupled(wc)


def test_paper_old_example_valid():
    world = World("""
        .  S0 S1 S2 .
       L0E .  .  .  .
        .  .  .  . L2W
       L1E .  .  .  .
        .  X  X  X  .""")
    wc = WorldCharacterizer(world, t_max=10)
    assert_fully_coupled(wc)
