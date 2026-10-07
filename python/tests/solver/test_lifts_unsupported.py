import lle
import pytest
from lle import World

TWO_FLOORS = """
S0 TU0 B0 S1
.  .   .  .
;
.  X   .  .
.  .   .  X
"""


def test_solve_raises_on_world_with_layers():
    with pytest.raises(NotImplementedError, match="several layers"):
        lle.solve(World(TWO_FLOORS), 10)


def test_solve_raises_on_single_layer_world_with_lifts():
    with pytest.raises(NotImplementedError, match="lifts"):
        lle.solve(World("S0 TU0 B0 X"), 10)


def test_characterize_raises_on_world_with_lifts():
    with pytest.raises(NotImplementedError):
        lle.characterize(World(TWO_FLOORS), 10)
