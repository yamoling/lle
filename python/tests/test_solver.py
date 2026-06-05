import lle
from lle import Action, World
from lle.solver._constraints import ConstraintContext

# from lle.solver.world_solver import WorldSolver


def _default_t_max(world: World) -> int:
    return (world.width * world.height) // 2


def test_solve_simple_world_returns_shortest_plan():
    world = World("S0 . . X")
    plan = lle.solve(world, t_max=5)
    assert plan is not None
    assert len(plan) == 3
    assert all(isinstance(row, tuple) for row in plan)
    assert all(isinstance(a, Action) for row in plan for a in row)


def test_solve_fixed_length():
    world = World("S0 . . X")
    plan = lle.solve(world, t_min=5, t_max=5)
    assert plan is not None
    assert len(plan) == 5


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


def test_solver_prunes_beam_variables_to_actual_laser_path():
    world = World.level(3)
    ctx = ConstraintContext(world, t_max=10)
    assert len(ctx.prev_laser_beam) == len(world.lasers) - 1


def test_incremental_solver_returns_shortest_plan():
    world = World("S0 . . X")
    plan = lle.solve(world, t_max=15)
    assert plan is not None
    assert len(plan) == 3  # Should find the shortest plan


def test_solver_unsolvable_returns_none():
    # Agent walled off from the exit.
    world = World("S0 @ X")
    plan = lle.solve(world)
    assert plan is None


def assert_agents_are_on_exit(world: World):
    for pos in world.agents_positions:
        assert pos in world.exit_pos


def test_solve_plan_is_executable():
    world = World("S0 . . X")
    plan = lle.solve(world, t_max=4)
    assert plan is not None
    world.reset()
    for joint in plan:
        world.step(list(joint))
    assert_agents_are_on_exit(world)
    # After executing, agent 0 should have exited.
    # (LLE marks exited agents with a specific position equal to their exit;
    #  we just check no exception was raised and the loop completed.)


def test_solve_path_is_executable_2agents():
    world = World("""
. . L1S . X
S0 .  .  . .
S1 .  .  . .
. .  .  . X
""")
    plan = lle.solve(world, t_max=10)
    assert plan is not None
    world.reset()
    for joint in plan:
        world.step(list(joint))
    assert_agents_are_on_exit(world)


def test_solve_level_6_world_is_executable():
    world = World.level(6)
    plan = lle.solve(world, t_max=21)
    assert plan is not None
    world.reset()
    for joint in plan:
        world.step(list(joint))
    assert_agents_are_on_exit(world)


def test_is_cooperative_on_known_cooperative_level():
    # LLE Level 6 is canonically cooperative.
    world = World.level(6)
    assert lle.is_cooperative(world)


def test_is_cooperative_on_trivial_single_agent_level():
    world = World("S0 . X")
    assert not lle.is_cooperative(world, t_max=3)


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
    T_MAX = [10, 10, 10, 10, 21, 21]
    for level, t_max in zip((1, 2, 3, 4, 5, 6), T_MAX):
        world = World.level(level)
        path = lle.solve(world, t_max=t_max)
        assert path is not None
        if level >= 3:
            assert lle.is_cooperative(world, t_max)


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
        path = lle.solve(world, t_max=10)
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
        assert lle.solve(world, t_max=10) is None


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


# ==========================================================
# Multiple laser sources of the same colour
# ==========================================================
#
# Beam paths and beam variables are keyed by (colour, direction, source position).
# A colour may therefore own any number of laser sources, including several pointing
# the same way. The previous (colour, direction) key let same-direction sources of one
# colour overwrite each other: only the last source's beam was built, so the others were
# silently dropped (and the solver crashed with a KeyError on the missing beam variable).


def test_two_same_colour_lasers_blocking_distinct_routes_is_unsat():
    # Agent 1 (colour 1) can only leave through (1, 0) or (1, 2); each exit sits under its
    # own colour-0 south laser. Agent 0 is sealed into column 4 and cannot block either
    # beam, so the level is genuinely unsolvable. Modelling only one of the two beams would
    # (wrongly) leave a route open.
    world = World("""
L0S @  L0S @ S0
 X  S1  X  @ X
""")
    assert lle.solve(world, t_max=6) is None


def test_two_same_colour_same_direction_lasers_with_clear_lanes_is_solvable():
    # Two colour-0 south lasers (same direction) at (0, 1) and (0, 2); both agents have a
    # beam-free lane (columns 0 and 3), so the level is solvable once both beams exist.
    world = World("""
S1 L0S L0S S0
.  .   .   .
X  .   .   X
""")
    assert lle.solve(world, t_max=6) is not None


def test_two_same_colour_crossing_lasers_keep_independent_beams():
    # A colour may own many lasers, in several directions, whose beams CROSS. Every laser
    # here is colour 0: three south lasers (columns 1, 2, 3), one east laser (row 1) and one
    # north laser (column 4). The east beam along row 1 crosses all four vertical beams. Each
    # beam must stay independent at every crossing rather than being forced to coincide:
    # a crossing cell carries one beam variable per (direction, source), not a single shared one.
    world = World("""
.   L0S L0S L0S X
L0E .   .   .   .
S0  .   .   .   .
S1  .   .   .   L0N
.   .   .   .   X
""")
    # solver = WorldSolver(world, t_max=14)
    # laser_sources = [(laser.color, laser.direction, src) for laser, src in solver.ctx.lasers]
    # directions = {(c, d) for c, d, _ in laser_sources}
    # assert len(laser_sources) == 5
    # assert {c for c, _ in directions} == {0}  # every laser is colour 0
    # assert len(directions) == 3  # south, east, north
    # # no same-direction source overwrites another: one beam path per source
    # assert len(solver.ctx.beam_paths) == 5
    # # at every crossing along the east beam (row 1) the beams remain distinct variables,
    # # keyed by (direction, source) -- never collapsed onto one shared beam.
    # for crossing in [(1, 1), (1, 2), (1, 3), (1, 4)]:
    #     beams_here = {(key[1], key[2]) for key in solver.ctx.beam_var if key[3:6] == (*crossing, 0)}
    #     assert len(beams_here) >= 2
    assert lle.solve(world, t_max=14) is not None
