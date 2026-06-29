from lle import World
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
