import lle
import pytest
from lle import World
from lle.characterization import WorldCharacterizer


def test_asymmetric_profile_with_independent_path_is_not_asymmetric():
    """When the shortest plan helps asymmetrically but an independent detour also exists
    within `t_max`, the world does not *require* asymmetric cooperation."""
    wc = WorldCharacterizer(
        World("""
     .  . S0 S1 . .
    L0E .  .  . @ .
     .  .  .  . . .
     .  .  .  . . .
     X  X  .  . . .
    """),
        t_max=10,
    )
    assert wc.is_independent()
    assert not wc.is_asymmetric()


def test_1_laser_world_requires_asymmetric_cooperation():
    world = World("""
     @  S0 S1
    L0E .  .
     @  X  X""")
    wc = lle.characterize(world, t_max=6)
    assert wc.is_solvable()
    assert wc.is_cooperative()
    assert wc.is_asymmetric()
    assert not wc.is_chained()
    assert not wc.is_mutual()
    assert not wc.is_interdependent()


def test_double_world_requires_asymmetric_cooperation():
    world = World("""
     @  S0 S1 @  @  S2 S3
    L0E .  .  @ L2E .  .
     @  X  X  @  @  X  X
    """)
    wc = lle.characterize(world, 6)
    assert wc.is_asymmetric()


def test_independent_world_is_not_asymmetric():
    world = World("""
    S0 . S1
     . . .
     X . X""")
    assert not lle.is_asymmetric(world, 6)


def test_pure_mutual_world_is_not_asymmetric():
    world = World("""
     S0 . . S1
    L0E . . .
     .  . . L1W
     X  . . X""")
    assert not lle.is_asymmetric(world, 6)


def test_unsolvable_world_raises_on_is_asymmetric():
    world = World("S0 @ X")
    with pytest.raises(ValueError):
        _ = WorldCharacterizer(world, t_max=10).is_asymmetric()


def test_one_way_cooperation_is_not_chained_or_interdependent():
    """A single required help edge is cooperative, but not a temporal chain or cycle."""
    world = World("""
 .  . S0 S1 . .
L0E .  .  . @ .
 .  .  .  . . .
 .  .  .  . . .
 X  X  .  . . .
""")
    for t_max in range(6, 12):
        is_cooperative = t_max <= 9
        wc = WorldCharacterizer(world, t_max)
        assert wc.is_solvable()
        assert wc.is_cooperative() == is_cooperative
        assert wc.is_asymmetric() == is_cooperative
        assert not wc.is_mutual()
        assert not wc.is_chained(2)
        assert not wc.is_chained(3)
        assert not wc.is_interdependent(2)
        assert not wc.is_interdependent(3)
