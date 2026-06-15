import lle
from lle import World
from lle.characterization.trajectory import profile_trajectory


def test_not_interdependent():
    """
    In the below world, the agents are not 3-interdependent because laser L1W
    can be avoided.
    """
    world = World("""
    S0  .  .  .  .  @ @
    S1  .  .  .  .  @ @
    S2 L0E .  .  .  @ @
    .  .  .  @  .  . .
    .  .  .  . L1W . .
    .  @ L2E .  .  . .
    .  @  @  .  X  X X
    """)
    plan = lle.solve(world, 16)
    assert plan is not None
    profile = profile_trajectory(world, plan)
    assert profile.is_cooperative
    assert profile.is_chained
    assert not profile.is_independent
    assert profile.is_interdependent(2)
    assert not profile.is_interdependent(3)
