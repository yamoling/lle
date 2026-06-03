from collections import defaultdict

from lle import World
from lle.solver._constraints import ConstraintContext, InitializationConstraints, LaserConstraints, MovementConstraints
from lle.solver.variable_factory import VariableFactory


def literal_name(var: VariableFactory, lit: int):
    name = var.name(lit)
    return ("not", name) if lit < 0 else name


def clause_names(var: VariableFactory, clauses: list[list[int]]):
    return [[literal_name(var, lit) for lit in clause] for clause in clauses]


def assert_has_clause(var: VariableFactory, clauses: list[list[int]], expected):
    assert list(expected) in clause_names(var, clauses)


def assert_not_has_variable(var: VariableFactory, kind: str, *prefix):
    for name in var.pool.obj2id:
        assert name[: 1 + len(prefix)] != (kind, *prefix)


def test_empty_level_with_one_agent_initializes_start_and_movement_clauses():
    world = World("S0 . X")
    var = VariableFactory()
    ctx = ConstraintContext(world, 0, 2)

    init_clauses = InitializationConstraints(var, ctx).generate(0)
    assert clause_names(var, init_clauses) == [[("agent", 0, 0, 0, 0)]]

    movement_clauses = MovementConstraints(var, ctx).generate(1)
    assert var.exists("agent", 0, 0, 0, 0)
    assert var.exists("agent", 0, 0, 0, 1)
    assert var.exists("agent", 0, 0, 1, 1)
    assert not var.exists("agent", 0, 0, 2, 0)

    # The agent can only be at the middle tile at t=1 if it came from its start at t=0.
    assert_has_clause(var, movement_clauses, [("not", ("agent", 0, 0, 1, 1)), ("agent", 0, 0, 0, 0)])
    # Staying at the start would make the exit unreachable by t_max, so it is forbidden,
    # not encoded as an empty unsatisfiable clause.
    assert_has_clause(var, movement_clauses, [("not", ("agent", 0, 0, 0, 1))])
    assert [] not in movement_clauses
    # Reachability filtering should prevent clauses from creating impossible t=0 variables.
    assert_not_has_variable(var, "agent", 0, 0, 1, 0)
    assert_not_has_variable(var, "agent", 0, 0, 2, 0)


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


def test_single_laser_unblocked_propagation_clauses():
    world = World("""
L0E . X
S0  . .
""")
    var = VariableFactory()
    ctx = ConstraintContext(world, 0, 2)
    source = world.laser_sources[0]

    laser_clauses = LaserConstraints(var, ctx).generate(0)
    assert var.exists("laser", source.laser_id, 0, 1, 0)
    assert var.exists("laser", source.laser_id, 0, 2, 0)

    assert clause_names(var, laser_clauses) == [
        [("not", ("laser", source.laser_id, 0, 0, 0)), ("laser", source.laser_id, 0, 1, 0)],
        [("laser", source.laser_id, 0, 0, 0), ("not", ("laser", source.laser_id, 0, 1, 0))],
        [("not", ("laser", source.laser_id, 0, 1, 0)), ("laser", source.laser_id, 0, 2, 0)],
        [("laser", source.laser_id, 0, 1, 0), ("not", ("laser", source.laser_id, 0, 2, 0))],
    ]


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

    LaserConstraints(var, ctx).generate(0)

    assert var.exists("laser", first.laser_id, 0, 0, 0)
    assert var.exists("laser", first.laser_id, 0, 1, 0)
    assert not var.exists("laser", first.laser_id, 0, 2, 0)
    assert var.exists("laser", second.laser_id, 0, 2, 0)
    assert var.exists("laser", second.laser_id, 0, 1, 0)
    assert not var.exists("laser", second.laser_id, 0, 0, 0)


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
