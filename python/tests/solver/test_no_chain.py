import pytest
from lle import World
from lle.characterization import WorldCharacterizer


def test_compute_shortest_path_without_chain_rejects_length_below_2():
    wc = WorldCharacterizer(World.level(1), t_max=10)
    for length in [1, 0, -1]:
        with pytest.raises(ValueError):
            wc.compute_shortest_path_without_chain(length)
