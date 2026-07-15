from lle.characterization import WorldCharacterizer

from .layouts import LEVEL_1


def test_eq_and_hash():
    """Equality and hashing depend on both the layout and horizon.

    @ai-generated
    """
    world = LEVEL_1.world()
    a = WorldCharacterizer(world, t_max=10)
    b = WorldCharacterizer(world, t_max=10)
    different_t_max = WorldCharacterizer(world, t_max=11)

    assert a == b
    assert hash(a) == hash(b)
    assert a != different_t_max
    assert a != "not a characterizer"
