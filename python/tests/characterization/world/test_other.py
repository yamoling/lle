from lle.characterization import WorldCharacterizer

from ...world_layouts import LEVEL_1, LEVEL_6


def test_eq_and_hash():
    """Equality and hashing depend on both the layout and horizon."""
    world = LEVEL_1.world()
    a = WorldCharacterizer(world, t_max=10)
    b = WorldCharacterizer(world, t_max=10)
    different_t_max = WorldCharacterizer(world, t_max=11)

    assert a == b
    assert hash(a) == hash(b)
    assert a != different_t_max
    assert a != "not a characterizer"


def test_shortest_non_mutual_path_solves_with_no_mutual_mode():
    """The lazily-computed non-mutual path is `None` when every solution is mutual."""
    wc = WorldCharacterizer(LEVEL_6.world(), t_max=21)
    assert wc.shortest_non_mutual_path is None
