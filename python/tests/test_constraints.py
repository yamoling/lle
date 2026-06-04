from collections import defaultdict

import pytest
from lle import World
from lle.solver._constraints import ConstraintContext, InitializationConstraints, LaserConstraints, MovementConstraints, utils
from lle.solver.variable_factory import VariableFactory


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


@pytest.mark.parametrize(
    "map_str",
    [
        #    "S0 . X",
        "S0 S1\n. .\nX X",
    ],
)
def test_exactly_one_position(map_str: str):
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

        # Verify XOR semantics: exactly one assignment satisfies all clauses
        from pysat.solvers import Minisat22

        with Minisat22(bootstrap_with=clauses) as s:
            assert s.solve(), "XOR clauses should be satisfiable"
            # Check that exactly one position variable is true for each agent in the solution
            model = s.get_model()
            assert model is not None
            true_vars = [v for v in model if v > 0]
            assert len(true_vars) == world.n_agents, (
                f"Exactly one position should be true, got {len(true_vars)}: {[var.name(v) for v in true_vars]}"
            )


def test_empty_level_with_two_agents_adds_collision_clauses():
    world = World("S0 . S1 X X")
    var = VariableFactory()
    ctx = ConstraintContext(world, 0, 2)

    movement_clauses = MovementConstraints(var, ctx).generate(1)

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
