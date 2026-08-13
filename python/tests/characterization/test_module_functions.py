"""Cover the module-level convenience wrappers in `lle.characterization`.

The `WorldCharacterizer` methods these wrap are already covered extensively
elsewhere; these tests only need to prove that each wrapper actually
delegates to its `WorldCharacterizer` counterpart.
"""

from __future__ import annotations

from lle.characterization import is_asymmetric, is_sequential, is_mutual

from ..world_layouts import LEVEL_1, LEVEL_5, LEVEL_6


def test_is_asymmetric_wrapper_matches_characterizer():
    """The module-level `is_asymmetric` delegates to `WorldCharacterizer.is_asymmetric`."""
    assert is_asymmetric(LEVEL_5.world(), t_max=21) is True
    assert is_asymmetric(LEVEL_5.world(), t_max=25) is False


def test_is_mutual_wrapper_matches_characterizer():
    """The module-level `is_mutual` delegates to `WorldCharacterizer.is_interdependent(2)`."""
    assert is_mutual(LEVEL_6.world(), t_max=21) is True
    assert is_mutual(LEVEL_1.world(), t_max=10) is False


def test_is_sequential_wrapper_matches_characterizer():
    """The module-level `is_sequential` delegates to `WorldCharacterizer.is_sequential`."""
    assert is_sequential(LEVEL_6.world(), t_max=21, length=2) is True
    assert is_sequential(LEVEL_6.world(), t_max=21, length=3) is False
