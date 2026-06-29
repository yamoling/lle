from lle import World
from lle.characterization.plan import profile_plan
from lle.solver import Solver


def test_mirrored_asymmetric():
    world = World("""
     @  S0 S1 @  @  S2 S3
    L0E .  .  @ L2E .  .
     @  X  X  @  @  X  X
    """)
    solver = Solver(world, 6)
    assert solver.solve() is not None
    assert solver.solve("no-asymmetric") is None


def test_independent_world_is_not_asymmetric():
    world = World("""
    S0 . S1
     . . .
     X . X""")
    solver = Solver(world, 6)
    assert solver.solve("no-asymmetric") is not None


def test_pure_mutual_world_has_non_asymmetric_solution():
    world = World("""
     S0 . . S1
    L0E . . .
     .  . . L1W
     X  . . X""")
    solver = Solver(world, 10)
    assert solver.solve() is not None
    assert solver.solve("no-asymmetric") is not None


def test_level_3_requires_agent_0_asymmetric_help():
    world = World.level(3)
    solver = Solver(world, 21)

    shortest = solver.solve()
    assert shortest is not None
    assert profile_plan(world, shortest).graph.asymmetric_edges() == {(0, 1)}
    assert solver.solve("no-asymmetric") is None


def test_level_5_requires_agent_1_asymmetric_help_at_tmax_21():
    world = World.level(5)
    solver = Solver(world, 21)

    shortest = solver.solve()
    assert shortest is not None
    assert profile_plan(world, shortest).graph.asymmetric_edges() == {(1, 0), (1, 2), (1, 3)}
    assert solver.solve("no-asymmetric") is None


def test_level_5_has_longer_non_asymmetric_solution_at_tmax_25():
    world = World.level(5)
    solver = Solver(world, 25)

    shortest = solver.solve()
    non_asymmetric = solver.solve("no-asymmetric")
    assert shortest is not None
    assert non_asymmetric is not None
    assert len(shortest) == 19
    assert len(non_asymmetric) == 24
