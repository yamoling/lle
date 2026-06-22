from copy import deepcopy

import lle
from lle import World
from lle.characterization.trajectory import TemporalDependencyGraph, profile_trajectory


def test_from_plan_matches_profile_trajectory_and_preserves_world():
    """`TemporalDependencyGraph.from_plan` should mirror trajectory profiling without mutating `world`."""
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

    world_before = deepcopy(world)
    graph = TemporalDependencyGraph.from_plan(plan, world)
    profile = profile_trajectory(deepcopy(world_before), plan)

    assert world.get_state() == world_before.get_state()
    assert graph.n_agents == profile.graph.n_agents
    assert graph.horizon == profile.graph.horizon
    assert graph.edges == profile.graph.edges


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
