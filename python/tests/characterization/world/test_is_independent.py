import pytest
from lle import World
from lle.characterization import WorldCharacterizer


def test_unsolvable_world_raises_on_is_independent():
    world = World("S0 @ X")
    with pytest.raises(ValueError):
        _ = WorldCharacterizer(world, t_max=10).is_independent()


def test_no_laser_world_is_independent():
    """Without any laser sources no blocking can occur: always independent."""
    world = World("S0 . S1\n.  .  .\nX  .  X")
    wc = WorldCharacterizer(world, t_max=6)
    assert wc.is_solvable()
    assert wc.is_independent()
    assert not wc.is_mutual()
