import lle
import pytest
from lle import World
from lle.characterization import WorldCharacterizer


def assert_canonical_asymmetric(wc: WorldCharacterizer):
    assert wc.is_asymmetric()
    assert wc.is_cooperative()
    assert not wc.is_chained(2)
    assert not wc.is_distributed(2)
    assert not wc.is_fully_coupled()
    assert not wc.is_independent()
    assert not wc.is_interdependent(2)


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


def test_level_3_is_asymmetric_because_agent_0_helps_without_being_helped():
    assert lle.is_asymmetric(World.level(3), 21)


def test_level_5_is_asymmetric_at_tmax_21_because_agent_1_helps_without_being_helped():
    assert lle.is_asymmetric(World.level(5), 21)


def test_level_5_is_not_asymmetric_when_longer_non_asymmetric_plan_is_allowed():
    """
    With a horizon of 25, agent 2 can block a laser for agent 1 because of the extra
    time. As a result, there is a cycle help(1, 2) and help(2, 1), and agent 1 is no
    longer helping without being helped, even though it is useless.
    """
    assert not lle.is_asymmetric(World.level(5), 25)


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
        assert not wc.is_chained(2)
        assert not wc.is_chained(3)
        assert not wc.is_interdependent(2)
        assert not wc.is_interdependent(3)
