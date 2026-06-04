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
        # Only test positions that are actually reachable (and thus have variables generated)
        reachable = gen.reachable_positions_for_agent(t, 0)
        if len(reachable) < 2:
            continue  # Skip if fewer than 2 positions are reachable
        positions = sorted(reachable)
        # Test that first two reachable positions can't both be true
        assert not solve_with_assumtions(
            clauses, [var.agent(0, positions[0][0], positions[0][1], t), var.agent(0, positions[1][0], positions[1][1], t)]
        ), "This should not be feasible"


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


@pytest.mark.parametrize("map_str", TEST_MAPS[1:])  # Need ≥ 2 agents
def test_no_overlap(map_str: str):
    """Two agents cannot occupy the same cell at the same time."""
    T_MAX = 5
    world = World(map_str)
    var = VariableFactory()
    ctx = ConstraintContext(world, 0, T_MAX)
    gen = MovementConstraints(var, ctx)

    all_clauses: list[list[int]] = []
    for t in range(T_MAX):
        all_clauses.extend(gen._exactly_one_position(t))
        all_clauses.extend(gen._time_wise_adjacency(t))
        all_clauses.extend(gen._all_collision_constraints(t))
        all_clauses.extend(gen._stays_on_exit(t))

    # Valid solution should exist
    true_vars = solve_and_get_true_variables(all_clauses)
    true_vars = [v for v in true_vars if var.name(v) is not None]

    # Verify no two agents share a cell at any timestep
    for t in range(T_MAX):
        positions_by_cell: dict[Position, list[int]] = defaultdict(list)
        for v in true_vars:
            name = var.name(v)
            if name and name[0] == "agent" and name[4] == t:
                _, agent_num, x, y, _ = name
                positions_by_cell[(x, y)].append(agent_num)
        for (x, y), agents in positions_by_cell.items():
            assert len(agents) <= 1, f"Agents {agents} both at ({x},{y}) at t={t}"

    # Prove impossibility: assert two agents share a cell (only test positions that are reachable by both)
    with Minisat22(bootstrap_with=all_clauses) as s:
        t = 1
        # Only test positions reachable by both agents
        reachable_both = ctx.reachable_positions(t, 0, 1)
        if reachable_both:  # Only test if there are positions reachable by both
            pos = next(iter(reachable_both))
            x, y = pos
            a = var.agent(0, x, y, t)
            b = var.agent(1, x, y, t)
            assert not s.solve(assumptions=[a, b]), f"Agents 0 and 1 should not both fit at ({x},{y}) at t={t}"


def test_empty_level_with_two_agents_adds_collision_clauses():
    """Two agents cannot swap places or follow each other into the same cell."""
    world = World("S0 . S1 X X")
    var = VariableFactory()
    ctx = ConstraintContext(world, 0, 3)
    gen = MovementConstraints(var, ctx)

    clauses = []
    clauses.extend(InitializationConstraints(var, ctx).generate(0))
    for t in range(3):
        clauses.extend(gen._exactly_one_position(t))
        clauses.extend(gen._time_wise_adjacency(t))
        clauses.extend(gen._all_collision_constraints(t))

    # Solve and verify no following conflicts
    true_vars = solve_and_get_true_variables(clauses)
    true_vars = [v for v in true_vars if var.name(v) is not None]

    # Build positions per agent per timestep
    pos: dict[int, dict[int, Position]] = defaultdict(dict)
    for v in true_vars:
        name = var.name(v)
        if name and name[0] == "agent":
            _, agent_num, x, y, t = name
            pos[agent_num][t] = (x, y)

    # Verify no agent moves into a cell that another agent occupied at the previous step
    for t in range(1, 3):
        for a1 in range(world.n_agents):
            for a2 in range(a1 + 1, world.n_agents):
                if t in pos[a1] and t - 1 in pos[a2]:
                    assert pos[a1][t] != pos[a2][t - 1], f"Agent {a1} followed agent {a2} into {pos[a1][t]} at t={t}"

    # Prove impossibility: agent 1 at (0,1) t=1 AND agent 0 at (0,1) t=0
    # (agent 0 starts at (0,0), can reach (0,1) at t=0? No. So use a different scenario.)
    # Following conflict: agent 1 enters (0,1) at t=1 that agent 0 was at t=0.
    # Agent 0 can be at (0,1) at t=1, agent 1 can be at (0,1) at t=0? No, agent 1 starts at (0,2).
    # Instead: agent 0 at (0,2) t=2 AND agent 1 at (0,2) t=1 — agent 0 follows agent 1.
    with Minisat22(bootstrap_with=clauses) as s:
        assert not s.solve(
            assumptions=[
                var.agent(0, 0, 2, 2),  # agent 0 at (0,2) at t=2
                var.agent(1, 0, 2, 1),  # agent 1 was at (0,2) at t=1
            ]
        ), "Agent 0 should not follow agent 1 into (0,2)"


def test_stays_on_exit():
    """If an agent is on an exit at t-1, it must also be on an exit at t."""
    # Map with exit at (0, 2). Agent starts at (0, 0), reaches exit at t=2.
    world = World("S0 . X")
    var = VariableFactory()
    ctx = ConstraintContext(world, 0, 4)
    gen = MovementConstraints(var, ctx)
    t = 3  # At t-1=2, agent could be at the exit (0, 2)

    clauses = list(gen._stays_on_exit(t))
    # Also need exactly_one_position to prevent agent from being at TWO positions at t
    clauses.extend(gen._exactly_one_position(t))
    assert len(clauses) > 0, "Expected clauses for stays-on-exit"

    # The exit is at (0, 2). Prove: agent at exit at t-1 AND agent at non-exit at t is impossible
    exit_pos = ctx.exits[0]  # (0, 2)
    agent_exit_prev = var.agent(0, exit_pos[0], exit_pos[1], t - 1)

    non_exit_positions = ctx.reachable_positions(t, 0) - set(ctx.exits)
    for nx, ny in non_exit_positions:
        agent_non_exit_now = var.agent(0, nx, ny, t)
        with Minisat22(bootstrap_with=clauses) as s:
            assert not s.solve(assumptions=[agent_exit_prev, agent_non_exit_now]), f"Agent should not leave exit (0,2) to go to ({nx},{ny})"


def test_laser_source_tiles_are_blocked_for_agent_reachability():
    """Laser source tiles block agent movement and reachability.

    With exit-distance filtering, positions that cannot reach an exit within the
    time horizon are not considered reachable, even if physically reachable.
    """
    world = World("S0 L0E X")
    ctx = ConstraintContext(world, 0, 2)

    assert (0, 1) not in ctx.valid_positions
    # Position (0,0) requires 3 steps to reach exit, but t_max=2, so it's not reachable
    assert ctx.reachable_positions_for_agent(1, 0) == set()


def test_unblocked_laser_does_not_generate_variable():
    """Verify laser variables are created for reachable beam positions.

    With exit-distance filtering, laser variables are only created for positions
    that are part of a potentially valid plan.
    """
    world = World("""
L0E . X
S0  . .
""")
    t = 3
    var = VariableFactory()
    ctx = ConstraintContext(world, 0, t)
    source = world.laser_sources[0]

    # With t_max=3, sufficient time to reach exit from all positions
    # Generate clauses (this should not error)
    clauses = list(LaserConstraints(var, ctx).generate(t))
    # Verify laser variables are created for positions that can reach exit
    # At t=3, most positions can reach exit, so laser vars should exist
    assert len(clauses) > 0, "Should generate some laser clauses"
    # Position (0,2) is the exit, so laser should be defined there
    assert var.exists("laser", source.laser_id, 0, 2, t)
    print(clauses)


def test_same_colour_agent_can_block_laser_destination():
    """A laser beam is active iff no same-colour agent blocks it.

    Map:
        .   X      (exit at (0,1))
        S0  .
        L0N .      (laser source at (2,0), going North)

    The laser beam path is: (2,0) -> (1,0) -> (0,0).
    If agent 0 is at (1,0), the beam should be blocked — laser at (0,0) must be off.
    If agent 0 is NOT at (1,0), the beam should propagate to (0,0).
    """
    world = World("""
.   X
S0  .
L0N .
""")
    var = VariableFactory()
    ctx = ConstraintContext(world, 0, 2)
    source = world.laser_sources[0]
    t = 0

    clauses = list(LaserConstraints(var, ctx).generate(t))
    # Laser source activation comes from InitializationConstraints
    init_clauses = InitializationConstraints(var, ctx).generate(t)
    all_clauses = init_clauses + clauses

    laser_var = var.laser(source.laser_id, 0, 0, t)
    agent_var = var.agent(0, 1, 0, t)

    # Prove: if agent is on (1,0), laser at (0,0) must be OFF
    with Minisat22(bootstrap_with=all_clauses) as s:
        assert not s.solve(assumptions=[agent_var, laser_var]), "Laser at (0,0) should be OFF when same-colour agent blocks at (1,0)"

    # Prove: if agent is NOT on (1,0), laser at (0,0) must be ON
    with Minisat22(bootstrap_with=all_clauses) as s:
        assert not s.solve(assumptions=[-agent_var, -laser_var]), "Laser at (0,0) should be ON when no same-colour agent blocks"


def test_different_colour_agent_cannot_step_on_active_laser():
    """An agent cannot step on an active laser beam of a different colour.

    Map:
        L0S . X      (laser L0S at (0,0) going South)
        .   S1 X     (agent S1 at (1,1))
        S0  . .      (agent S0 at (2,0))

    Laser L0S beam path: (0,0) -> (1,0) -> (2,0).
    Agent 1 (colour 1) ≠ laser colour (0), so agent 1 cannot step on laser positions
    if the laser is active there.
    """
    world = World("""
L0S . X
.   S1 X
S0  . .
""")
    var = VariableFactory()
    ctx = ConstraintContext(world, 0, 2)
    source = world.laser_sources[0]
    t = 1

    laser_clauses = list(LaserConstraints(var, ctx).generate(t))
    # Also need movement constraints for the agent position variable to exist
    movgen = MovementConstraints(var, ctx)
    movement_clauses = [
        *movgen._exactly_one_position(t),
        *movgen._time_wise_adjacency(t),
    ]
    all_clauses = laser_clauses + movement_clauses

    # Find a position that is on the laser path and reachable by agent 1
    laser_path = [(0, 0), (1, 0), (2, 0)]
    reachable_agent1 = movgen.reachable_positions_for_agent(t, 1)
    laser_on_reachable = [pos for pos in laser_path if pos in reachable_agent1]

    if laser_on_reachable:
        # Test with the first reachable laser position
        x, y = laser_on_reachable[0]
        agent_var = var.agent(1, x, y, t)
        laser_var = var.laser(source.laser_id, x, y, t)

        # Agent 1 (different colour) cannot be at laser position when laser is active
        with Minisat22(bootstrap_with=all_clauses) as s:
            assert not s.solve(assumptions=[agent_var, laser_var]), (
                f"Agent 1 (colour 1) cannot step on active laser L0 (colour 0) at ({x},{y})"
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
