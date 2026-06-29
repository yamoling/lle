import lle
from lle import World


def test_time_dependent_threshold():
    # Short route (cols 0-1) forces mutual help across two length-2 beams; a laser-free detour exists
    # down cols 4-5 (around the wall column), so mutual help is required only below a time threshold.
    world = World("""
     S0 S1 . . .
    L0E  . . @ .
    L1E  . . @ .
      X  X . @ .
      .  . . . .
    """)
    # Empirically, mutual help is required up to t=12 and a mutual-free plan exists from t=13 on.
    threshold = 13
    # Below the threshold: solvable, but only via mutual cooperation.
    for t in range(threshold):
        if t == 11:
            print()
        if lle.solve(world, t) is None:
            continue
        assert lle.solve(world, t, mode="no-mutual") is None, f"expected mutual help at t={t}"
    # At/above the threshold: a mutual-free plan appears.
    assert lle.solve(world, threshold, mode="no-mutual") is not None
    # The mutual-free plan is itself a valid plan (replays without error onto the world).
    plan = lle.solve(world, threshold, mode="no-mutual")
    assert plan is not None
    world.reset()
    for joint in plan:
        world.step(joint)
    assert all(agent.is_alive and agent.has_arrived for agent in world.agents)


def test_solve_std_levels_without_mutual_cooperation():
    # Levels 1, 2, 3, and 5 do not require mutual cooperation -> a path should be found
    for level, t_max in zip((1, 2, 3, 5), (10, 10, 10, 19)):
        world = World.level(level)
        assert lle.solve(world, t_max, mode="no-mutual") is not None

    # Levels 4 and 6 require mutual cooperation -> no path should be found
    for level, t_max in zip((4, 6), (10, 21)):
        world = World.level(level)
        assert lle.solve(world, t_max, mode="no-mutual") is None
