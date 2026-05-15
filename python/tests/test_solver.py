import lle
from lle import Action, World


def _default_t_max(world: World) -> int:
    return (world.width * world.height) // 2


def test_solve_simple_world_returns_plan_with_t_max_length():
    world = World("S0 . . X")
    plan = lle.solve(world, t_max=5)
    assert plan is not None
    assert len(plan) == 5
    assert all(isinstance(row, tuple) for row in plan)
    assert all(isinstance(a, Action) for row in plan for a in row)


def test_solve_unsolvable_returns_none():
    # Agent walled off from the exit.
    world = World("S0 @ X")
    assert lle.solve(world, t_max=10) is None


def test_solve_default_t_max():
    # 2x2 grid: agent at (0,0), exit at (1,1). default t_max = (2*2)//2 = 2, which is sufficient.
    world = World("S0 .\n.  X")
    plan = lle.solve(world)  # default t_max
    assert plan is not None
    assert len(plan) == _default_t_max(world)


def test_solve_plan_is_executable():
    world = World("S0 . . X")
    plan = lle.solve(world, t_max=4)
    assert plan is not None
    world.reset()
    for joint in plan:
        world.step(list(joint))
    # After executing, agent 0 should have exited.
    # (LLE marks exited agents with a specific position equal to their exit;
    #  we just check no exception was raised and the loop completed.)


def test_is_cooperative_on_known_cooperative_level():
    # LLE Level 6 is canonically cooperative.
    world = World.level(6)
    assert lle.is_cooperative(world) is True


def test_is_cooperative_on_trivial_single_agent_level():
    world = World("S0 . X")
    assert lle.is_cooperative(world) is False


def test_simple_solvable():
    world = World("""
 . . . . X
S0 . . . .
S1 . . . .
 . . . . X
""")
    path = lle.solve(world)
    assert path is not None
    assert not lle.is_cooperative(world)


def test_standard_levels_solvable():
    for level in (1, 2, 3, 4, 5, 6):
        world = World.level(level)
        path = lle.solve(world, 25)
        assert path is not None
        if level >= 3:
            assert lle.is_cooperative(world)


def test_simple_solvable_cooprative():
    # All worlds are solvable in 10 steps at most
    worlds = [
        """
 . . L1S . X
S0 .  .  . .
S1 .  .  . .
 . .  .  . X""",
        """
.  . L0S . X
S0 .  .  . .
S1 .  .  . .
.  .  .  . X""",
        """
.   .  L0S . X
S0  .   .  . .
S1  .   .  . .
.  L1N  .  . X""",
        """
. L1S L0S .
S0  .   .  X
S1  .   .  X""",
    ]
    for ws in worlds:
        world = World(ws)
        path = lle.solve(world, 10)
        assert path is not None
        assert lle.is_cooperative(world, 10)


def test_not_solvable():
    worlds = [
        """
 . L1S  .  .
S0  .   .  X
S1  .   .  X
 .  .  L1N ."""
    ]
    for ws in worlds:
        world = World(ws)
        assert lle.solve(world, 10) is None


def test_solvable_non_cooperative():
    worlds = [
        """
.  X L0S . X
S0 .  .  . .
S1 .  .  . .
.  X  .  . X
""",
    ]
    for ws in worlds:
        world = World(ws)
        assert not lle.is_cooperative(world, 10)
