from collections import defaultdict
from typing import Any, Generator

import pytest
from lle import World
from lle.solver._constraints import ConstraintContext, InitializationConstraints, LaserConstraints, MovementConstraints, utils
from lle.solver.variable_factory import VariableFactory
from lle.types import Position
from pysat.solvers import Minisat22

TEST_MAPS = [
    "S0 . X",
    "S0 S1\n. .\nX X",
    "S0 S1 S2 S3\n. . . .\nX X X X",
]


def solve_with_assumtions(clauses: list | Generator[list, Any, None], assumptions: list):
    with Minisat22(bootstrap_with=list(clauses)) as s:
        return s.solve(assumptions=assumptions)


def solve_and_get_true_variables(clauses: list | Generator[list, Any, None]):
    with Minisat22(bootstrap_with=list(clauses)) as s:
        assert s.solve(), "Non statisfiable formula"
        model = s.get_model()
        assert model is not None
        return [v for v in model if v > 0]


def test_empty_level_with_one_agent_initializes_start_and_movement_clauses():
    world = World("S0 . X")
    var = VariableFactory()
    ctx = ConstraintContext(world, 0, 10)

    init_clauses = InitializationConstraints(var, ctx).generate(0)
    assert len(init_clauses) == world.n_agents + len(world.lasers)
    assert var.exists("agent", 0, 0, 0, 0)
    # Until t=0, there should not be any consideration of agent 0 being in (0, 1) or (0, 2)
    # because we know it is impossible.
    assert not var.exists("agent", 0, 0, 1, 0)
    assert not var.exists("agent", 0, 0, 2, 0)

    t = 1
    movgen = MovementConstraints(var, ctx)
    movement_clauses = list(movgen._exactly_one_position(t))
    # Two possibilities: remain on spot or move east
    assert len(movement_clauses) == 2
    assert var.exists("agent", 0, 0, 0, t)
    assert var.exists("agent", 0, 0, 1, t)
    assert not var.exists("agent", 0, 0, 2, t)


@pytest.mark.parametrize("map_str", TEST_MAPS)
def test_exactly_one_position(map_str: str):
    """
    This test verifies that satisfiable solutions output by the solver indeed verifies what is expected;
    """
    T_MAX = 10
    world = World(map_str)
    var = VariableFactory()
    ctx = ConstraintContext(world, 0, T_MAX)
    movgen = MovementConstraints(var, ctx)

    for t in range(1, T_MAX):
        clauses = list(movgen._exactly_one_position(t))
        # Agent 0 can reach (0,0) and (0,1) by any t >= 1, so XOR produces clauses
        assert len(clauses) > 0, f"Expected clauses at t={t}"

        # Collect all agent position variables referenced in clauses
        agent_vars = set()
        for clause in clauses:
            for lit in clause:
                agent_vars.add(abs(lit))

        # Verify all referenced variables are agent position vars for agent 0 at time t
        for x, y in ctx.reachable_positions(t, 0):
            vid = var.agent(0, x, y, t)
            assert vid in agent_vars, f"Variable for agent 0 at ({x},{y},t={t}) should appear in clauses"

        # Verify exactly-one semantics: exactly one assignment satisfies all clauses
        true_agent_vars = solve_and_get_true_variables(clauses)
        var_names = [var.name(v) for v in true_agent_vars]
        # The variable name is of shape ("agent", agent_num, x, y, t)
        agent_nums = set(v[1] for v in var_names if v is not None)
        assert len(agent_nums) == world.n_agents, f"Exactly one position per agent should be true, got {len(true_agent_vars)}: {var_names}"


@pytest.mark.parametrize("map_str", TEST_MAPS)
def test_exactly_one_position_wrong_assumtions_fail(map_str: str):
    """
    This test verifies that satisfiable solutions output by the solver indeed verifies what is expected;
    """
    T_MAX = 10
    world = World(map_str)
    var = VariableFactory()
    ctx = ConstraintContext(world, 0, T_MAX)
    gen = MovementConstraints(var, ctx)

    for t in range(1, T_MAX):
        clauses = list(gen._exactly_one_position(t))
        assert not solve_with_assumtions(clauses, [var.agent(0, 0, 0, t), var.agent(0, 0, 1, t)]), "This should not be feasible"


@pytest.mark.parametrize("map_str", TEST_MAPS)
def test_time_wise_adjacency(map_str: str):
    """Verify that solver-produced trajectories have valid movement: consecutive
    positions for each agent must be adjacent (or the same cell).

    This test combines both _exactly_one_position and _time_wise_adjacency clauses,
    since neither alone is sufficient:
      - exactly_one_position: one position per timestep, but no movement validation
      - time_wise_adjacency: enforces adjacency, but without exactly-one the solver
        could pick multiple positions per timestep
    """
    T_MAX = 10
    world = World(map_str)
    var = VariableFactory()
    ctx = ConstraintContext(world, 0, T_MAX)
    gen = MovementConstraints(var, ctx)

    # Collect all clauses across all timesteps
    all_clauses: list[list[int]] = []
    for t in range(T_MAX):
        all_clauses.extend(gen._exactly_one_position(t))
        all_clauses.extend(gen._time_wise_adjacency(t))

    # Solve
    true_vars = solve_and_get_true_variables(all_clauses)
    # Filter out auxiliary variables (from CardEnc.atmost) not tracked in the vpool
    true_vars = [v for v in true_vars if var.name(v) is not None]
    var_names = [var.name(v) for v in true_vars]

    # Build trajectory: agent -> {t -> (x, y)}
    trajectory: dict[int, dict[int, Position]] = defaultdict(dict)
    for name in var_names:
        assert name is not None, "None variable while it is returned by the sovler"
        kind, agent_num, x, y, t = name
        if kind == "agent":
            trajectory[agent_num][t] = (x, y)

    # Verify: every agent has exactly one position per timestep
    for agent_num in range(world.n_agents):
        assert agent_num in trajectory, f"Agent {agent_num} has no positions in solution"
        for t in range(T_MAX):
            assert t in trajectory[agent_num], f"Agent {agent_num} missing position at t={t}"

        # Verify consecutive positions are neighbors (Manhattan distance ≤ 1)
        for t in range(1, T_MAX):
            prev_pos = trajectory[agent_num][t - 1]
            curr_pos = trajectory[agent_num][t]
            dx, dy = abs(curr_pos[0] - prev_pos[0]), abs(curr_pos[1] - prev_pos[1])
            assert dx + dy <= 1, f"Agent {agent_num} movements are not adjacent. {prev_pos} (t={t - 1}) -> {curr_pos} (t={t})"


@pytest.mark.parametrize("map_str", TEST_MAPS)
def test_no_overlap(map_str: str):
    T_MAX = 10
    world = World(map_str)
    var = VariableFactory()
    ctx = ConstraintContext(world, 0, T_MAX)
    gen = MovementConstraints(var, ctx)
    clauses = []
    for t in range(T_MAX):
        clauses.extend(gen._no_overlap(t))


def test_empty_level_with_two_agents_adds_collision_clauses():
    world = World("S0 . S1 X X")
    var = VariableFactory()
    ctx = ConstraintContext(world, 0, 2)

    gen = MovementConstraints(var, ctx)

    assert var.exists("agent", 0, 0, 1, 1)
    assert var.exists("agent", 1, 0, 1, 1)
    assert_has_clause(
        var,
        movement_clauses,
        [("not", ("agent", 0, 0, 1, 1)), ("not", ("agent", 1, 0, 1, 1))],
    )


def test_laser_source_tiles_are_blocked_for_agent_reachability():
    world = World("S0 L0E X")
    ctx = ConstraintContext(world, 0, 2)

    assert (0, 1) not in ctx.valid_positions
    assert ctx.reachable_positions_for_agent(1, 0) == {(0, 0)}


def test_unblocked_laser_does_not_generate_variable():
    world = World("""
L0E . X
S0  . .
""")
    t = 3
    var = VariableFactory()
    ctx = ConstraintContext(world, 0, t)
    source = world.laser_sources[0]

    clauses = LaserConstraints(var, ctx).generate(t)
    assert var.exists("laser", source.laser_id, 0, 1, t)
    assert var.exists("laser", source.laser_id, 0, 2, t)
    print(clauses)


def test_same_colour_agent_can_block_laser_destination():
    world = World("""
.   X
S0  .
L0N .
""")
    var = VariableFactory()
    ctx = ConstraintContext(world, 0, 2)
    source = world.laser_sources[0]

    laser_clauses = LaserConstraints(var, ctx).generate(0)

    assert_has_clause(
        var,
        laser_clauses,
        [
            ("not", ("laser", source.laser_id, 2, 0, 0)),
            ("agent", source.agent_id, 1, 0, 0),
            ("laser", source.laser_id, 1, 0, 0),
        ],
    )
    assert_has_clause(
        var,
        laser_clauses,
        [("not", ("agent", source.agent_id, 1, 0, 0)), ("not", ("laser", source.laser_id, 1, 0, 0))],
    )
    assert_has_clause(
        var,
        laser_clauses,
        [("laser", source.laser_id, 2, 0, 0), ("not", ("laser", source.laser_id, 1, 0, 0))],
    )


def test_different_colour_agent_cannot_step_on_active_laser():
    world = World("""
L0S . X
.   S1 X
S0  . .
""")
    var = VariableFactory()
    ctx = ConstraintContext(world, 0, 2)
    source = world.laser_sources[0]

    laser_clauses = LaserConstraints(var, ctx).generate(1)

    assert_has_clause(
        var,
        laser_clauses,
        [("not", ("agent", 1, 1, 0, 1)), ("not", ("laser", source.laser_id, 1, 0, 1))],
    )


def test_two_lasers_stop_at_each_others_source_tiles():
    world = World("""
L0E . L1W X X
S0  . S1  . .
""")
    var = VariableFactory()
    ctx = ConstraintContext(world, 0, 2)
    first, second = world.laser_sources
    t = 2
    LaserConstraints(var, ctx).generate(t)

    assert not var.exists("laser", first.laser_id, 0, 0, t)
    assert var.exists("laser", first.laser_id, 0, 1, t)
    assert not var.exists("laser", first.laser_id, 0, 2, t)
    assert not var.exists("laser", second.laser_id, 0, 2, t)
    assert var.exists("laser", second.laser_id, 0, 1, t)
    assert not var.exists("laser", second.laser_id, 0, 0, t)


def test_multiple_same_colour_same_direction_lasers_get_independent_beams():
    # Two colour-0 south lasers in different columns share (colour, direction); each must
    # still keep its own beam, keyed by its distinct source position.
    world = World("""
.  L0S .  L0S .
S0 .   .  .   S1
X  .   .  .   X
""")
    var = VariableFactory()
    ctx = ConstraintContext(world, 0, 10)
    constraints = LaserConstraints(var, ctx)
    constraints.generate(1)
    # Laser source with id 0 is at (0, 1)
    s1 = world.source_at((0, 1)).laser_id
    assert var.exists("laser", s1, 1, 1, 1)
    assert not var.exists("laser", s1, 1, 3, 1)
    # Laser source with id 1 is at (0, 3)
    s2 = world.source_at((0, 3)).laser_id
    assert var.exists("laser", s2, 1, 3, 1)
    assert not var.exists("laser", s2, 1, 1, 1)


def test_two_same_colour_crossing_lasers_keep_variables():
    # A colour may own many lasers, in several directions, whose beams CROSS. Every laser
    # here is colour 0: three south lasers (columns 1, 2, 3), one east laser (row 1) and one
    # north laser (column 4). The east beam along row 1 crosses all four vertical beams. Each
    # beam must stay independent at every crossing rather than being forced to coincide:
    # a crossing cell carries one beam variable per (direction, source), not a single shared one.
    world = World("""
.   L0S L0S  X
L0E  .   .    .
S0   .   .    .
S1   .   .    .
.    .   .    X
""")
    var = VariableFactory()
    ctx = ConstraintContext(world, 0, 20)
    constraints = LaserConstraints(var, ctx)
    constraints.generate(10)
    # Make sure that there exist different variables for crossing lasers at i=1
    n_vars_at = defaultdict(lambda: 0)
    for kind, *rest in var.pool.obj2id:
        if kind != "laser":
            continue
        laser_id, i, j, t = rest
        # Count the lasers variables at the intersection
        n_vars_at[i, j, t] += 1
    for (i, j, t), count in n_vars_at.items():
        # Count the actual number of superimposed beam tiles at each position. Source
        # tiles also get laser variables now, but they are not crossings and should not
        # be part of this beam-superposition assertion.
        n_lasers_superimposed = sum(laser.pos == (i, j) for laser in world.lasers)
        if n_lasers_superimposed == 0:
            continue
        assert count == n_lasers_superimposed


def test_implies_expansion():
    """In CNF, implication expands to (¬a ∨ b)"""
    a = 1
    b = 2
    clauses = list(utils.implies(a, b))
    assert len(clauses) == 1
    assert len(clauses[0]) == 2
    clause = sorted(clauses[0])
    assert clause == [-a, b]


def test_equality_expansion():
    """Equality expands to (a -> b) ∧ (b -> a). In CNF: (¬a ∨ b) ∧ (¬b ∨ a)"""
    a = 1
    b = 2
    clauses = [sorted(c) for c in utils.equals(a, b)]
    assert len(clauses) == 2
    expected = [[-a, b], [-b, a]]
    for c in expected:
        assert c in clauses


def test_xor_expansion():
    """XOR expands to (¬a ∨ ¬b) ∧ (a ∨ b)"""
    a = 1
    b = 2
    clauses = [sorted(c) for c in utils.xor(a, b)]
    assert len(clauses) == 2
    expected = [[a, b], [-b, -a]]
    for c in expected:
        assert c in clauses
